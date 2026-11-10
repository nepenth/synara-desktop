//! What kind of desktop session the app is running in.
//!
//! One place answers "is this Wayland", "is this KDE" and "does a tiling
//! window manager own the window". The renderer reads the last answer from
//! the bridge (`windowEnvironment`) to decide whether to draw its own title
//! strip; see docs/future-projects/tiling-mode-and-bottom-bar.

use std::env;

/// Who arranges the app's windows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowEnvironment {
    /// A stacking desktop: the user moves and sizes the window.
    Floating,
    /// A tiling window manager places the window; window controls and a
    /// drag strip have no job.
    Tiling,
}

impl WindowEnvironment {
    pub fn as_str(self) -> &'static str {
        match self {
            WindowEnvironment::Floating => "floating",
            WindowEnvironment::Tiling => "tiling",
        }
    }
}

/// Sockets and signatures that only a running tiling compositor exports.
const TILING_SESSION_VARIABLES: &[&str] = &[
    "HYPRLAND_INSTANCE_SIGNATURE",
    "SWAYSOCK",
    "NIRI_SOCKET",
    "I3SOCK",
];

/// Names a tiling compositor reports in `XDG_CURRENT_DESKTOP` (a
/// colon-separated list) or `XDG_SESSION_DESKTOP`, lower-cased.
const TILING_DESKTOP_NAMES: &[&str] = &[
    "hyprland",
    "sway",
    "niri",
    "i3",
    "river",
    "dwl",
    "bspwm",
    "qtile",
    "awesome",
    "xmonad",
    "herbstluftwm",
];

fn present(lookup: &impl Fn(&str) -> Option<String>, key: &str) -> bool {
    lookup(key).is_some_and(|value| !value.trim().is_empty())
}

fn names_tiling_desktop(value: &str) -> bool {
    value
        .split(':')
        .map(|name| name.trim().to_ascii_lowercase())
        .any(|name| TILING_DESKTOP_NAMES.contains(&name.as_str()))
}

/// Pure detection over an environment lookup, so tests need no process
/// environment. `os` is `std::env::consts::OS`.
pub fn window_environment_from(
    os: &str,
    lookup: impl Fn(&str) -> Option<String>,
) -> WindowEnvironment {
    if os != "linux" {
        return WindowEnvironment::Floating;
    }
    if TILING_SESSION_VARIABLES
        .iter()
        .any(|key| present(&lookup, key))
    {
        return WindowEnvironment::Tiling;
    }
    let named = ["XDG_CURRENT_DESKTOP", "XDG_SESSION_DESKTOP"]
        .iter()
        .filter_map(|key| lookup(key))
        .any(|value| names_tiling_desktop(&value));
    if named {
        WindowEnvironment::Tiling
    } else {
        WindowEnvironment::Floating
    }
}

pub fn window_environment() -> WindowEnvironment {
    window_environment_from(env::consts::OS, |key| env::var(key).ok())
}

pub fn is_kde() -> bool {
    env::var("XDG_CURRENT_DESKTOP")
        .map(|value| value.to_ascii_lowercase().contains("kde"))
        .unwrap_or(false)
}

pub fn is_wayland() -> bool {
    if env::var("WAYLAND_DISPLAY").is_ok() {
        return true;
    }

    env::var("XDG_SESSION_TYPE")
        .map(|value| value.eq_ignore_ascii_case("wayland"))
        .unwrap_or(false)
}

pub fn is_kde_wayland_session() -> bool {
    is_kde() && is_wayland()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn detect(os: &str, vars: &[(&str, &str)]) -> WindowEnvironment {
        let map: HashMap<String, String> = vars
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect();
        window_environment_from(os, |key| map.get(key).cloned())
    }

    #[test]
    fn hyprland_signature_means_tiling() {
        assert_eq!(
            detect("linux", &[("HYPRLAND_INSTANCE_SIGNATURE", "abc_123")]),
            WindowEnvironment::Tiling
        );
    }

    #[test]
    fn compositor_sockets_mean_tiling() {
        for key in ["SWAYSOCK", "NIRI_SOCKET", "I3SOCK"] {
            assert_eq!(
                detect("linux", &[(key, "/run/user/1000/sock")]),
                WindowEnvironment::Tiling,
                "{key}"
            );
        }
    }

    #[test]
    fn empty_socket_variable_is_ignored() {
        assert_eq!(
            detect("linux", &[("SWAYSOCK", " ")]),
            WindowEnvironment::Floating
        );
    }

    #[test]
    fn desktop_name_is_matched_per_entry_and_case_insensitively() {
        assert_eq!(
            detect("linux", &[("XDG_CURRENT_DESKTOP", "Hyprland")]),
            WindowEnvironment::Tiling
        );
        assert_eq!(
            detect("linux", &[("XDG_CURRENT_DESKTOP", "sway:wlroots")]),
            WindowEnvironment::Tiling
        );
        assert_eq!(
            detect("linux", &[("XDG_SESSION_DESKTOP", "niri")]),
            WindowEnvironment::Tiling
        );
    }

    #[test]
    fn stacking_desktops_stay_floating() {
        for desktop in [
            "KDE",
            "GNOME",
            "ubuntu:GNOME",
            "COSMIC",
            "XFCE",
            "pop:GNOME",
            "",
        ] {
            assert_eq!(
                detect("linux", &[("XDG_CURRENT_DESKTOP", desktop)]),
                WindowEnvironment::Floating,
                "{desktop}"
            );
        }
        assert_eq!(detect("linux", &[]), WindowEnvironment::Floating);
    }

    #[test]
    fn a_name_that_only_contains_a_tiling_name_does_not_match() {
        assert_eq!(
            detect("linux", &[("XDG_CURRENT_DESKTOP", "i3status-desktop")]),
            WindowEnvironment::Floating
        );
    }

    #[test]
    fn other_systems_are_always_floating() {
        for os in ["macos", "windows"] {
            assert_eq!(
                detect(os, &[("HYPRLAND_INSTANCE_SIGNATURE", "abc")]),
                WindowEnvironment::Floating,
                "{os}"
            );
        }
    }
}
