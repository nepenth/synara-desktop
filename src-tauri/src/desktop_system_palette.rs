//! The desktop's own colour palette, for "Follow system theme colors".
//!
//! Two providers:
//! - Omarchy (Linux) keeps the active theme as files under
//!   `~/.local/state/omarchy/current/theme` (`~/.config/omarchy/current/theme`
//!   on older installs) and replaces that directory when the user picks
//!   another theme. It gives a background, a text colour and an accent, and
//!   the renderer derives its surfaces from them
//!   (`utils/themeExternalPalette.ts`).
//! - macOS has no theme background or text colour to follow, only the accent
//!   colour from System Settings. Its palette carries the accent alone and
//!   the renderer keeps its own theme's surfaces.
//!
//! The renderer re-themes on the change event. Where no provider applies the
//! command answers `null` and the setting stays hidden.

// The theme-file parsers serve Linux alone; elsewhere they are built for the
// tests.
#![cfg_attr(not(any(target_os = "linux", test)), allow(dead_code))]

use serde::Serialize;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemPalette {
    /// Which provider read it: `omarchy` or `macos`.
    pub source: String,
    /// `#rrggbb`. Absent, with `foreground`, when the desktop shares only an
    /// accent (macOS): the app's own theme then keeps its surfaces and text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreground: Option<String>,
    /// `#rrggbb`.
    pub accent: String,
    /// The terminal colours 0–15 when the theme defines them all.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub colors: Vec<String>,
}

/// `#rgb`, `#rrggbb`, `#rrggbbaa`, `0xrrggbb` or bare `rrggbb` as `#rrggbb`.
fn normalize_hex(value: &str) -> Option<String> {
    let value = value.trim().trim_matches(|ch| ch == '"' || ch == '\'');
    let digits = value
        .strip_prefix('#')
        .or_else(|| value.strip_prefix("0x"))
        .or_else(|| value.strip_prefix("0X"))
        .unwrap_or(value);
    if !digits.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return None;
    }
    let digits = digits.to_ascii_lowercase();
    match digits.len() {
        3 => Some(format!(
            "#{}",
            digits.chars().flat_map(|ch| [ch, ch]).collect::<String>()
        )),
        6 => Some(format!("#{digits}")),
        8 => Some(format!("#{}", &digits[..6])),
        _ => None,
    }
}

/// The `key = value` pairs of a simple TOML file as `section.key` (or `key`
/// at the top level). Enough for colour files: no arrays, inline tables or
/// multi-line strings.
fn toml_pairs(text: &str) -> HashMap<String, String> {
    let mut pairs = HashMap::new();
    let mut section = String::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            section = name.trim().trim_matches('[').trim_matches(']').to_owned();
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim().trim_matches('"');
        let value = value.trim();
        // A quoted value ends at its closing quote; an unquoted one at a comment.
        let value = match value.chars().next() {
            Some(quote @ ('"' | '\'')) => value[1..]
                .split_once(quote)
                .map(|(inner, _)| inner)
                .unwrap_or(&value[1..]),
            _ => value.split('#').next().unwrap_or("").trim(),
        };
        let full_key = if section.is_empty() {
            key.to_owned()
        } else {
            format!("{section}.{key}")
        };
        pairs.insert(full_key.to_ascii_lowercase(), value.to_owned());
    }
    pairs
}

fn first_color(pairs: &HashMap<String, String>, keys: &[&str]) -> Option<String> {
    keys.iter()
        .filter_map(|key| pairs.get(*key))
        .find_map(|value| normalize_hex(value))
}

fn terminal_colors(pairs: &HashMap<String, String>, keys: &[[&str; 3]; 16]) -> Vec<String> {
    let colors: Vec<String> = keys
        .iter()
        .filter_map(|candidates| first_color(pairs, candidates))
        .collect();
    if colors.len() == 16 {
        colors
    } else {
        Vec::new()
    }
}

/// Each terminal colour under its Alacritty name, its ANSI name and the
/// semantic name Omarchy's `colors.toml` uses (which has no black or white:
/// the background, foreground and muted colours stand in, as Omarchy's own
/// `omarchy-theme-color` resolves them).
const TERMINAL_COLOR_KEYS: [[&str; 3]; 16] = [
    ["colors.normal.black", "color0", "background"],
    ["colors.normal.red", "color1", "red"],
    ["colors.normal.green", "color2", "green"],
    ["colors.normal.yellow", "color3", "yellow"],
    ["colors.normal.blue", "color4", "blue"],
    ["colors.normal.magenta", "color5", "magenta"],
    ["colors.normal.cyan", "color6", "cyan"],
    ["colors.normal.white", "color7", "foreground"],
    ["colors.bright.black", "color8", "muted"],
    ["colors.bright.red", "color9", "bright_red"],
    ["colors.bright.green", "color10", "bright_green"],
    ["colors.bright.yellow", "color11", "bright_yellow"],
    ["colors.bright.blue", "color12", "bright_blue"],
    ["colors.bright.magenta", "color13", "bright_magenta"],
    ["colors.bright.cyan", "color14", "bright_cyan"],
    ["colors.bright.white", "color15", "bright_foreground"],
];

/// A palette from a theme's colour file: either a flat `colors.toml`
/// (`background`, `foreground`, `accent`, then `red`, `bright_red`… or
/// `color0`…`color15`) or a terminal theme in Alacritty's layout
/// (`[colors.primary]`, `[colors.normal]`). The accent falls back to the
/// terminal's blue.
pub fn palette_from_toml(source: &str, text: &str) -> Option<SystemPalette> {
    let pairs = toml_pairs(text);
    let background = first_color(
        &pairs,
        &["background", "bg", "colors.primary.background", "color0"],
    )?;
    let foreground = first_color(
        &pairs,
        &["foreground", "fg", "colors.primary.foreground", "color7"],
    )?;
    let accent = first_color(
        &pairs,
        &[
            "accent",
            "blue",
            "colors.normal.blue",
            "color4",
            "colors.bright.blue",
            "color12",
        ],
    )?;
    Some(SystemPalette {
        source: source.to_owned(),
        background: Some(background),
        foreground: Some(foreground),
        accent,
        colors: terminal_colors(&pairs, &TERMINAL_COLOR_KEYS),
    })
}

/// sRGB components (nominally 0–1) as `#rrggbb`. Values outside the range
/// are clamped, as a wide-gamut accent converted to sRGB can overshoot;
/// `None` for a component that is not a number.
#[cfg(any(target_os = "macos", test))]
fn srgb_hex(red: f64, green: f64, blue: f64) -> Option<String> {
    let channel = |value: f64| -> Option<u8> {
        if value.is_nan() {
            return None;
        }
        // Clamped to 0–255 first, so the cast cannot truncate.
        Some((value.clamp(0.0, 1.0) * 255.0).round() as u8)
    };
    Some(format!(
        "#{:02x}{:02x}{:02x}",
        channel(red)?,
        channel(green)?,
        channel(blue)?
    ))
}

/// A palette that is an accent colour and nothing else.
#[cfg(any(target_os = "macos", test))]
fn accent_palette(source: &str, red: f64, green: f64, blue: f64) -> Option<SystemPalette> {
    Some(SystemPalette {
        source: source.to_owned(),
        background: None,
        foreground: None,
        accent: srgb_hex(red, green, blue)?,
        colors: Vec::new(),
    })
}

/// The accent colour chosen in System Settings › Appearance
/// (`NSColor.controlAccentColor`), converted to sRGB. The colour is dynamic:
/// macOS gives a slightly different shade in light and dark appearance, so
/// this also changes when the appearance does.
#[cfg(target_os = "macos")]
fn macos_accent_palette() -> Option<SystemPalette> {
    use objc2_app_kit::{NSColor, NSColorSpace};

    // The watch calls this from a plain thread, which has no pool of its own.
    objc2::rc::autoreleasepool(|_| {
        let accent =
            NSColor::controlAccentColor().colorUsingColorSpace(&NSColorSpace::sRGBColorSpace())?;
        accent_palette(
            "macos",
            accent.redComponent(),
            accent.greenComponent(),
            accent.blueComponent(),
        )
    })
}

/// Colour files of an Omarchy theme, most specific first.
#[cfg(any(target_os = "linux", test))]
const OMARCHY_THEME_FILES: &[&str] = &["colors.toml", "alacritty.toml"];

#[cfg(any(target_os = "linux", test))]
fn omarchy_palette_in(theme_dir: &std::path::Path) -> Option<SystemPalette> {
    OMARCHY_THEME_FILES.iter().find_map(|name| {
        let text = std::fs::read_to_string(theme_dir.join(name)).ok()?;
        palette_from_toml("omarchy", &text)
    })
}

/// Whether a provider applies on this desktop at all (worth watching).
pub fn provider_available() -> bool {
    #[cfg(target_os = "linux")]
    {
        omarchy_theme_dir().is_some()
    }
    #[cfg(target_os = "macos")]
    {
        true
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        false
    }
}

/// Where Omarchy keeps the theme in use. It moved from the config directory
/// to the state directory (`~/.local/state/omarchy/current/theme`), so both
/// are looked at, the current place first.
#[cfg(target_os = "linux")]
fn omarchy_theme_dir() -> Option<std::path::PathBuf> {
    let from_env = |name: &str| {
        std::env::var_os(name)
            .filter(|dir| !dir.is_empty())
            .map(std::path::PathBuf::from)
    };
    let roots = [
        from_env("XDG_STATE_HOME").or_else(dirs::state_dir),
        from_env("XDG_CONFIG_HOME").or_else(dirs::config_dir),
    ];
    roots
        .into_iter()
        .flatten()
        .map(|root| root.join("omarchy").join("current"))
        .find(|current| current.is_dir())
        .map(|current| current.join("theme"))
}

/// The palette of the desktop the app runs on, if a provider knows it.
pub fn current_system_palette() -> Option<SystemPalette> {
    #[cfg(target_os = "linux")]
    {
        omarchy_theme_dir().and_then(|dir| omarchy_palette_in(&dir))
    }
    #[cfg(target_os = "macos")]
    {
        macos_accent_palette()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALACRITTY: &str = r##"
# Tokyo Night
[colors.primary]
background = "#1a1b26"
foreground = '#a9b1d6'

[colors.normal]
black = "#32344a"
red = "#f7768e"
green = "#9ece6a"
yellow = "#e0af68"
blue = "#7aa2f7"
magenta = "#ad8ee6"
cyan = "#449dab"
white = "#787c99"

[colors.bright]
black = "#444b6a"
red = "#ff7a93"
green = "#b9f27c"
yellow = "#ff9e64"
blue = "#7da6ff"
magenta = "#bb9af7"
cyan = "#0db9d7"
white = "#acb0d0"

[colors.selection]
background = "#7aa2f7"
"##;

    #[test]
    fn hex_forms_normalize_to_six_lowercase_digits() {
        assert_eq!(normalize_hex("#1A1B26").as_deref(), Some("#1a1b26"));
        assert_eq!(normalize_hex("0x1a1b26").as_deref(), Some("#1a1b26"));
        assert_eq!(normalize_hex("1a1b26").as_deref(), Some("#1a1b26"));
        assert_eq!(normalize_hex("\"#abc\"").as_deref(), Some("#aabbcc"));
        assert_eq!(normalize_hex("#1a1b26ff").as_deref(), Some("#1a1b26"));
        for bad in ["", "#12", "#12345", "blue", "rgb(1,2,3)", "#gggggg"] {
            assert_eq!(normalize_hex(bad), None, "{bad}");
        }
    }

    #[test]
    fn an_alacritty_theme_gives_background_foreground_and_blue_as_accent() {
        let palette = palette_from_toml("omarchy", ALACRITTY).unwrap();
        assert_eq!(palette.source, "omarchy");
        assert_eq!(palette.background.as_deref(), Some("#1a1b26"));
        assert_eq!(palette.foreground.as_deref(), Some("#a9b1d6"));
        assert_eq!(palette.accent, "#7aa2f7");
        assert_eq!(palette.colors.len(), 16);
        assert_eq!(palette.colors[0], "#32344a");
        assert_eq!(palette.colors[15], "#acb0d0");
    }

    #[test]
    fn a_flat_colors_file_is_read_and_its_accent_wins() {
        let text = r##"
accent = "#c4a7e7"   # iris
background = "#191724"
foreground = "#e0def4"
color4 = "#31748f"
"##;
        let palette = palette_from_toml("omarchy", text).unwrap();
        assert_eq!(palette.accent, "#c4a7e7");
        assert_eq!(palette.background.as_deref(), Some("#191724"));
        assert_eq!(palette.foreground.as_deref(), Some("#e0def4"));
        // Fewer than sixteen terminal colours: none are reported.
        assert!(palette.colors.is_empty());
    }

    /// The `colors.toml` Omarchy writes: semantic names, no `colorN` keys.
    #[test]
    fn an_omarchy_semantic_colors_file_gives_all_sixteen_terminal_colours() {
        let text = r##"
mode = "dark"

accent = "#7aa2f7"
selection = "#292e42"
muted = "#414868"

background = "#1a1b26"
foreground = "#a9b1d6"
bright_foreground = "#c0caf5"

red = "#f7768e"
yellow = "#e0af68"
green = "#9ece6a"
cyan = "#449dab"
blue = "#7aa2f7"
magenta = "#ad8ee6"

bright_red = "#ff7a93"
bright_yellow = "#ff9e64"
bright_green = "#b9f27c"
bright_cyan = "#0db9d7"
bright_blue = "#7da6ff"
bright_magenta = "#bb9af7"
"##;
        let palette = palette_from_toml("omarchy", text).unwrap();
        assert_eq!(palette.accent, "#7aa2f7");
        assert_eq!(palette.background.as_deref(), Some("#1a1b26"));
        assert_eq!(palette.foreground.as_deref(), Some("#a9b1d6"));
        assert_eq!(palette.colors.len(), 16);
        assert_eq!(palette.colors[0], "#1a1b26");
        assert_eq!(palette.colors[1], "#f7768e");
        assert_eq!(palette.colors[7], "#a9b1d6");
        assert_eq!(palette.colors[8], "#414868");
        assert_eq!(palette.colors[15], "#c0caf5");
    }

    #[test]
    fn short_and_ansi_names_stand_in_for_background_and_foreground() {
        let palette = palette_from_toml(
            "omarchy",
            "bg = \"#000000\"\nfg = \"#ffffff\"\nblue = \"#0000ff\"\n",
        )
        .unwrap();
        assert_eq!(palette.background.as_deref(), Some("#000000"));
        assert_eq!(palette.foreground.as_deref(), Some("#ffffff"));
        assert_eq!(palette.accent, "#0000ff");
    }

    #[test]
    fn a_file_without_the_three_colours_is_no_palette() {
        assert_eq!(palette_from_toml("omarchy", ""), None);
        assert_eq!(
            palette_from_toml("omarchy", "[colors.primary]\nbackground = \"#000000\"\n"),
            None
        );
        assert_eq!(
            palette_from_toml(
                "omarchy",
                "background = \"nope\"\nforeground = \"#ffffff\"\naccent = \"#ff0000\"\n"
            ),
            None
        );
    }

    #[test]
    fn the_theme_directory_prefers_colors_toml_and_falls_back_to_the_terminal_theme() {
        let dir = std::env::temp_dir().join(format!("synara-palette-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(omarchy_palette_in(&dir), None);

        std::fs::write(dir.join("alacritty.toml"), ALACRITTY).unwrap();
        assert_eq!(omarchy_palette_in(&dir).unwrap().accent, "#7aa2f7");

        std::fs::write(
            dir.join("colors.toml"),
            "accent = \"#ff0000\"\nbackground = \"#000000\"\nforeground = \"#ffffff\"\n",
        )
        .unwrap();
        assert_eq!(omarchy_palette_in(&dir).unwrap().accent, "#ff0000");

        // An unreadable colors.toml does not hide the terminal theme.
        std::fs::write(dir.join("colors.toml"), "not a palette").unwrap();
        assert_eq!(omarchy_palette_in(&dir).unwrap().accent, "#7aa2f7");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_payload_matches_the_renderer_contract() {
        let palette = palette_from_toml("omarchy", ALACRITTY).unwrap();
        let value = serde_json::to_value(&palette).unwrap();
        assert_eq!(value["source"], "omarchy");
        assert_eq!(value["background"], "#1a1b26");
        assert_eq!(value["foreground"], "#a9b1d6");
        assert_eq!(value["accent"], "#7aa2f7");
        assert_eq!(value["colors"].as_array().unwrap().len(), 16);
    }

    #[test]
    fn srgb_components_become_a_hex_colour() {
        // The macOS blue accent in light appearance.
        assert_eq!(srgb_hex(0.0, 0.478, 1.0).as_deref(), Some("#007aff"));
        assert_eq!(srgb_hex(0.0, 0.0, 0.0).as_deref(), Some("#000000"));
        assert_eq!(srgb_hex(1.0, 1.0, 1.0).as_deref(), Some("#ffffff"));
        assert_eq!(srgb_hex(0.5, 0.25, 0.75).as_deref(), Some("#8040bf"));
        // A wide-gamut colour can leave the sRGB range; it is clamped.
        assert_eq!(srgb_hex(1.2, -0.1, 0.5).as_deref(), Some("#ff0080"));
        assert_eq!(
            srgb_hex(f64::INFINITY, f64::NEG_INFINITY, 0.0).as_deref(),
            Some("#ff0000")
        );
        assert_eq!(srgb_hex(f64::NAN, 0.0, 0.0), None);
        assert_eq!(srgb_hex(0.0, 0.0, f64::NAN), None);
    }

    #[test]
    fn an_accent_only_palette_carries_no_background_or_text_colour() {
        let palette = accent_palette("macos", 0.0, 0.478, 1.0).unwrap();
        assert_eq!(palette.source, "macos");
        assert_eq!(palette.accent, "#007aff");
        assert_eq!(palette.background, None);
        assert_eq!(palette.foreground, None);
        assert_eq!(
            serde_json::to_value(&palette).unwrap(),
            serde_json::json!({ "source": "macos", "accent": "#007aff" })
        );
        assert_eq!(accent_palette("macos", f64::NAN, 0.0, 0.0), None);
    }
}
