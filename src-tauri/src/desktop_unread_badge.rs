//! Linux unread count for the dash/dock and notification-area tray icon.
//!
//! GNOME's dash tracks mapped windows, not processes. Unity's `libunity`
//! launcher badge is a no-op unless Unity itself is running, so this module
//! paints a Slack-style count onto the tray icon and broadcasts the Unity
//! LauncherEntry signal that Ubuntu Dock, Dash to Dock, and KDE listen for.

// Overlay painting and Unity path helpers are called from the Linux tray.
// macOS still compiles the same functions for unit tests.
#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

#[cfg(target_os = "linux")]
use std::collections::HashMap;

const BADGE_RED: [u8; 4] = [220, 50, 47, 255];
const BADGE_WHITE: [u8; 4] = [255, 255, 255, 255];
const MAX_BADGE_COUNT: i64 = 9_999;

/// 3×5 digits `0-9` plus `+`, packed row-major.
const GLYPH_3X5: [[u8; 15]; 11] = [
    [1, 1, 1, 1, 0, 1, 1, 0, 1, 1, 0, 1, 1, 1, 1], // 0
    [0, 1, 0, 1, 1, 0, 0, 1, 0, 0, 1, 0, 1, 1, 1], // 1
    [1, 1, 1, 0, 0, 1, 1, 1, 1, 1, 0, 0, 1, 1, 1], // 2
    [1, 1, 1, 0, 0, 1, 0, 1, 1, 0, 0, 1, 1, 1, 1], // 3
    [1, 0, 1, 1, 0, 1, 1, 1, 1, 0, 0, 1, 0, 0, 1], // 4
    [1, 1, 1, 1, 0, 0, 1, 1, 1, 0, 0, 1, 1, 1, 1], // 5
    [1, 1, 1, 1, 0, 0, 1, 1, 1, 1, 0, 1, 1, 1, 1], // 6
    [1, 1, 1, 0, 0, 1, 0, 1, 0, 0, 1, 0, 0, 1, 0], // 7
    [1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1], // 8
    [1, 1, 1, 1, 0, 1, 1, 1, 1, 0, 0, 1, 1, 1, 1], // 9
    [0, 0, 0, 0, 1, 0, 1, 1, 1, 0, 1, 0, 0, 0, 0], // +
];

pub fn clamp_unread_badge_count(count: i64) -> i64 {
    count.clamp(0, MAX_BADGE_COUNT)
}

pub fn unread_badge_label(count: i64) -> Option<String> {
    let count = clamp_unread_badge_count(count);
    if count == 0 {
        None
    } else if count > 99 {
        Some("99+".to_owned())
    } else {
        Some(count.to_string())
    }
}

/// Desktop file IDs the running window may be grouped under.
///
/// Arch installs `Synara.desktop`; Tauri's Unity helper uses the Cargo package
/// name `synara.desktop`. Emit both so dash/dock badges match either id.
pub fn unity_launcher_app_uris() -> [&'static str; 2] {
    [
        "application://Synara.desktop",
        "application://synara.desktop",
    ]
}

/// GLib `g_str_hash`, which libunity uses in the LauncherEntry object path.
pub fn glib_str_hash(value: &str) -> u32 {
    let mut hash: u32 = 5381;
    for byte in value.bytes() {
        hash = hash
            .wrapping_shl(5)
            .wrapping_add(hash)
            .wrapping_add(u32::from(byte));
    }
    hash
}

pub fn unity_launcher_object_path(app_uri: &str) -> String {
    format!(
        "/com/canonical/unity/launcherentry/{}",
        glib_str_hash(app_uri)
    )
}

pub fn overlay_unread_badge(rgba: &[u8], width: u32, height: u32, count: i64) -> Option<Vec<u8>> {
    let label = unread_badge_label(count)?;
    if width == 0 || height == 0 {
        return None;
    }
    let expected = (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)?;
    if rgba.len() < expected {
        return None;
    }
    let mut out = rgba[..expected].to_vec();
    paint_unread_badge(&mut out, width, height, &label);
    Some(out)
}

fn paint_unread_badge(pixels: &mut [u8], width: u32, height: u32, label: &str) {
    let min_side = width.min(height) as f32;
    let diameter = (min_side * 0.46).clamp(10.0, min_side);
    let radius = diameter / 2.0;
    let extra = ((label.len().saturating_sub(1) as f32) * radius * 0.72).max(0.0);
    let badge_w = diameter + extra;
    let badge_h = diameter;
    let margin = (min_side * 0.05).clamp(1.0, 8.0);
    let cx = width as f32 - margin - badge_w / 2.0;
    let cy = margin + badge_h / 2.0;
    let left = (cx - badge_w / 2.0).floor() as i32;
    let right = (cx + badge_w / 2.0).ceil() as i32;
    let top = (cy - badge_h / 2.0).floor() as i32;
    let bottom = (cy + badge_h / 2.0).ceil() as i32;

    for y in top..=bottom {
        for x in left..=right {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let dx = (px - cx).abs() - (badge_w / 2.0 - radius);
            let dx = dx.max(0.0);
            let dy = py - cy;
            let cover = (radius + 0.5 - dx.hypot(dy)).clamp(0.0, 1.0);
            if cover <= 0.0 {
                continue;
            }
            blend_px(
                pixels,
                width,
                height,
                x,
                y,
                BADGE_RED,
                (cover * 255.0).round() as u8,
            );
        }
    }

    draw_label(pixels, width, height, cx, cy, badge_h, label);
}

fn draw_label(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    cx: f32,
    cy: f32,
    badge_h: f32,
    label: &str,
) {
    let glyphs: Vec<[u8; 15]> = label
        .chars()
        .filter_map(|ch| match ch {
            '0'..='9' => Some(GLYPH_3X5[(ch as u8 - b'0') as usize]),
            '+' => Some(GLYPH_3X5[10]),
            _ => None,
        })
        .collect();
    if glyphs.is_empty() {
        return;
    }

    let scale = ((badge_h / 11.0).floor() as i32).clamp(1, 6);
    let glyph_w = 3 * scale;
    let glyph_h = 5 * scale;
    let gap = scale.max(1);
    let total_w = glyphs.len() as i32 * glyph_w + (glyphs.len() as i32 - 1) * gap;
    let origin_x = (cx - total_w as f32 / 2.0).round() as i32;
    let origin_y = (cy - glyph_h as f32 / 2.0).round() as i32;

    for (index, glyph) in glyphs.iter().enumerate() {
        let gx = origin_x + index as i32 * (glyph_w + gap);
        for (cell, on) in glyph.iter().copied().enumerate() {
            if on == 0 {
                continue;
            }
            let cx_cell = (cell % 3) as i32;
            let cy_cell = (cell / 3) as i32;
            for oy in 0..scale {
                for ox in 0..scale {
                    blend_px(
                        pixels,
                        width,
                        height,
                        gx + cx_cell * scale + ox,
                        origin_y + cy_cell * scale + oy,
                        BADGE_WHITE,
                        255,
                    );
                }
            }
        }
    }
}

fn blend_px(pixels: &mut [u8], width: u32, height: u32, x: i32, y: i32, color: [u8; 4], alpha: u8) {
    if x < 0 || y < 0 {
        return;
    }
    let x = x as u32;
    let y = y as u32;
    if x >= width || y >= height {
        return;
    }
    let index = ((y * width + x) * 4) as usize;
    if index + 3 >= pixels.len() {
        return;
    }
    let src_a = (u16::from(color[3]) * u16::from(alpha)) / 255;
    let dst_a = 255 - src_a;
    for channel in 0..3 {
        let src = u16::from(color[channel]) * src_a;
        let dst = u16::from(pixels[index + channel]) * dst_a;
        pixels[index + channel] = ((src + dst) / 255) as u8;
    }
    pixels[index + 3] = 255;
}

#[cfg(target_os = "linux")]
pub fn emit_unity_launcher_badge(count: i64) {
    let count = clamp_unread_badge_count(count);
    tauri::async_runtime::spawn_blocking(move || {
        if let Err(error) = emit_unity_launcher_badge_blocking(count) {
            eprintln!("[synara] Unity launcher badge update failed: {error}");
        }
    });
}

#[cfg(target_os = "linux")]
fn owned_unity_value(value: zbus::zvariant::Value<'static>) -> zbus::zvariant::OwnedValue {
    value
        .try_to_owned()
        .expect("Unity badge fields are copy types")
}

/// `a{sv}` so Dash to Dock / Ubuntu Dock / KDE can decode count + visibility.
#[cfg(target_os = "linux")]
fn unity_launcher_properties(count: i64) -> HashMap<&'static str, zbus::zvariant::OwnedValue> {
    let mut properties = HashMap::new();
    properties.insert("count", owned_unity_value(count.into()));
    properties.insert("count-visible", owned_unity_value((count > 0).into()));
    properties
}

#[cfg(target_os = "linux")]
fn emit_unity_launcher_badge_blocking(count: i64) -> zbus::Result<()> {
    let connection = zbus::blocking::Connection::session()?;
    for app_uri in unity_launcher_app_uris() {
        connection.emit_signal(
            None::<&str>,
            unity_launcher_object_path(app_uri),
            "com.canonical.Unity.LauncherEntry",
            "Update",
            &(app_uri, unity_launcher_properties(count)),
        )?;
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn emit_unity_launcher_badge(_count: i64) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn badge_label_hides_zero_and_caps_at_99_plus() {
        assert_eq!(unread_badge_label(0), None);
        assert_eq!(unread_badge_label(-4), None);
        assert_eq!(unread_badge_label(1), Some("1".to_owned()));
        assert_eq!(unread_badge_label(99), Some("99".to_owned()));
        assert_eq!(unread_badge_label(100), Some("99+".to_owned()));
        assert_eq!(unread_badge_label(50_000), Some("99+".to_owned()));
    }

    #[test]
    fn overlay_requires_a_positive_count_and_preserves_size() {
        let width = 32;
        let height = 32;
        let src = vec![12u8; (width * height * 4) as usize];
        assert!(overlay_unread_badge(&src, width, height, 0).is_none());
        let out = overlay_unread_badge(&src, width, height, 7).expect("badge");
        assert_eq!(out.len(), src.len());
    }

    #[test]
    fn overlay_paints_red_in_the_top_right() {
        let width = 32u32;
        let height = 32u32;
        let mut src = vec![0u8; (width * height * 4) as usize];
        for px in src.chunks_exact_mut(4) {
            px[0] = 24;
            px[1] = 24;
            px[2] = 24;
            px[3] = 255;
        }
        let out = overlay_unread_badge(&src, width, height, 3).expect("badge");
        let mut found_red = false;
        for y in 0..height / 2 {
            for x in width / 2..width {
                let i = ((y * width + x) * 4) as usize;
                if out[i] > 180 && out[i + 1] < 90 && out[i + 2] < 90 && out[i + 3] > 200 {
                    found_red = true;
                }
            }
        }
        assert!(found_red);
    }

    #[test]
    fn unity_object_path_uses_glib_str_hash() {
        let uri = "application://synara.desktop";
        assert_eq!(
            unity_launcher_object_path(uri),
            format!("/com/canonical/unity/launcherentry/{}", glib_str_hash(uri))
        );
        assert!(unity_launcher_app_uris()[0].ends_with("Synara.desktop"));
        assert!(unity_launcher_app_uris()[1].ends_with("synara.desktop"));
        assert_eq!(glib_str_hash(""), 5381);
        assert_ne!(
            glib_str_hash("application://Synara.desktop"),
            glib_str_hash("application://synara.desktop")
        );
    }

    #[test]
    fn unity_update_body_is_string_plus_variant_dict() {
        let source = include_str!("desktop_unread_badge.rs");
        assert!(source.contains("HashMap<&'static str, zbus::zvariant::OwnedValue>"));
        assert!(source.contains("\"count-visible\""));
        assert!(source.contains("com.canonical.Unity.LauncherEntry"));
    }
}
