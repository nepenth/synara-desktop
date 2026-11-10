//! The desktop's own colour palette, for "Follow system theme colors".
//!
//! One provider so far: Omarchy, which keeps the active theme as files under
//! `~/.config/omarchy/current/theme` and swaps that link when the user picks
//! another theme. The renderer derives its surfaces and accent from the three
//! colours here (`utils/themeExternalPalette.ts`) and re-themes on the change
//! event. Where no provider applies the command answers `null` and the
//! setting stays hidden.

// Only Linux has a provider so far; elsewhere the parsers are built for the
// tests alone.
#![cfg_attr(not(any(target_os = "linux", test)), allow(dead_code))]

use serde::Serialize;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemPalette {
    /// Which provider read it, e.g. `omarchy`.
    pub source: String,
    /// `#rrggbb`.
    pub background: String,
    pub foreground: String,
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

fn terminal_colors(pairs: &HashMap<String, String>, keys: &[[&str; 2]; 16]) -> Vec<String> {
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

const ALACRITTY_COLOR_KEYS: [[&str; 2]; 16] = [
    ["colors.normal.black", "color0"],
    ["colors.normal.red", "color1"],
    ["colors.normal.green", "color2"],
    ["colors.normal.yellow", "color3"],
    ["colors.normal.blue", "color4"],
    ["colors.normal.magenta", "color5"],
    ["colors.normal.cyan", "color6"],
    ["colors.normal.white", "color7"],
    ["colors.bright.black", "color8"],
    ["colors.bright.red", "color9"],
    ["colors.bright.green", "color10"],
    ["colors.bright.yellow", "color11"],
    ["colors.bright.blue", "color12"],
    ["colors.bright.magenta", "color13"],
    ["colors.bright.cyan", "color14"],
    ["colors.bright.white", "color15"],
];

/// A palette from a theme's colour file: either a flat `colors.toml`
/// (`background`, `foreground`, `accent`, `color0`…`color15`) or a terminal
/// theme in Alacritty's layout (`[colors.primary]`, `[colors.normal]`).
/// The accent falls back to the terminal's blue.
pub fn palette_from_toml(source: &str, text: &str) -> Option<SystemPalette> {
    let pairs = toml_pairs(text);
    let background = first_color(&pairs, &["background", "colors.primary.background"])?;
    let foreground = first_color(&pairs, &["foreground", "colors.primary.foreground"])?;
    let accent = first_color(
        &pairs,
        &[
            "accent",
            "colors.normal.blue",
            "color4",
            "colors.bright.blue",
            "color12",
        ],
    )?;
    Some(SystemPalette {
        source: source.to_owned(),
        background,
        foreground,
        accent,
        colors: terminal_colors(&pairs, &ALACRITTY_COLOR_KEYS),
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
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

#[cfg(target_os = "linux")]
fn omarchy_theme_dir() -> Option<std::path::PathBuf> {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|dir| !dir.is_empty())
        .map(std::path::PathBuf::from)
        .or_else(dirs::config_dir)?;
    let current = config.join("omarchy").join("current");
    current.is_dir().then(|| current.join("theme"))
}

/// The palette of the desktop the app runs on, if a provider knows it.
pub fn current_system_palette() -> Option<SystemPalette> {
    #[cfg(target_os = "linux")]
    {
        omarchy_theme_dir().and_then(|dir| omarchy_palette_in(&dir))
    }
    #[cfg(not(target_os = "linux"))]
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
        assert_eq!(palette.background, "#1a1b26");
        assert_eq!(palette.foreground, "#a9b1d6");
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
        assert_eq!(palette.background, "#191724");
        assert_eq!(palette.foreground, "#e0def4");
        // Fewer than sixteen terminal colours: none are reported.
        assert!(palette.colors.is_empty());
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
}
