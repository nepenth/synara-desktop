//! Linux unread count for the dash/dock and a dot on the notification-area icon.
//!
//! GNOME's dash tracks mapped windows, not processes. Unity's `libunity`
//! launcher badge is a no-op unless Unity itself is running, so this module
//! paints a red dot onto the tray icon and broadcasts the Unity LauncherEntry
//! count signal that supporting docks listen for. COSMIC's app-list does not
//! currently consume that signal, so its dock icon needs upstream badge support.

// Overlay painting and Unity path helpers are called from the Linux tray.
// macOS still compiles the same functions for unit tests.
#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

#[cfg(target_os = "linux")]
use std::collections::HashMap;

const BADGE_RED: [u8; 4] = [220, 50, 47, 255];
const MAX_BADGE_COUNT: i64 = 9_999;

pub fn clamp_unread_badge_count(count: i64) -> i64 {
    count.clamp(0, MAX_BADGE_COUNT)
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

/// Notification-area icons render at roughly 16–24 px. A count is unreadable
/// there, so keep the number for the launcher and paint only an attention dot.
pub fn overlay_unread_dot(rgba: &[u8], width: u32, height: u32, count: i64) -> Option<Vec<u8>> {
    if count <= 0 || width == 0 || height == 0 {
        return None;
    }
    let expected = (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)?;
    if rgba.len() < expected {
        return None;
    }
    let mut out = rgba[..expected].to_vec();
    let side = width.min(height) as f32;
    let radius = (side * 0.21).clamp(3.0, side / 2.0);
    let margin = (side * 0.04).clamp(1.0, 6.0);
    let cx = width as f32 - margin - radius;
    let cy = margin + radius;
    for y in (cy - radius).floor() as i32..=(cy + radius).ceil() as i32 {
        for x in (cx - radius).floor() as i32..=(cx + radius).ceil() as i32 {
            let cover =
                (radius + 0.5 - (x as f32 + 0.5 - cx).hypot(y as f32 + 0.5 - cy)).clamp(0.0, 1.0);
            if cover > 0.0 {
                blend_px(
                    &mut out,
                    width,
                    height,
                    x,
                    y,
                    BADGE_RED,
                    (cover * 255.0) as u8,
                );
            }
        }
    }
    Some(out)
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
    fn panel_dot_is_visible_at_16px_and_does_not_encode_the_count() {
        let src = vec![0u8; 16 * 16 * 4];
        assert!(overlay_unread_dot(&src, 16, 16, 0).is_none());
        let one = overlay_unread_dot(&src, 16, 16, 1).expect("dot");
        let many = overlay_unread_dot(&src, 16, 16, 500).expect("dot");
        assert_eq!(one, many);
        assert!(one
            .chunks_exact(4)
            .any(|pixel| pixel[0] > 180 && pixel[1] < 90));
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
