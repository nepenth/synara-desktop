//! How the Linux web view draws: on the CPU ("safe") or with the graphics
//! card ("gpu"), the "Hardware acceleration" switch in Settings.
//!
//! GTK and WebKitGTK read the choice from environment variables when the
//! process starts, so it is decided before the app builds its window and a
//! change applies on the next start. The saved choice is a one-word file,
//! `rendering-mode`, in the app's config directory; no file means the user
//! has not chosen, and the package's default applies. macOS and Windows web
//! views are always GPU-accelerated and never read it.
//!
//! This module has no Tauri types; `desktop_rendering_host` holds the commands
//! and the startup call.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Environment variable that picks the mode for one start and wins over the
/// saved setting.
pub const RENDERING_ENV: &str = "SYNARA_RENDERING";
/// Environment variable a package's launcher sets to say what applies while
/// the user has not chosen. Without it nothing is changed.
pub const RENDERING_DEFAULT_ENV: &str = "SYNARA_RENDERING_DEFAULT";
/// Name of the preference file in the app's config directory.
pub const PREFERENCE_FILE_NAME: &str = "rendering-mode";

const WEBKIT_DISABLE_DMABUF_RENDERER: &str = "WEBKIT_DISABLE_DMABUF_RENDERER";
const WEBKIT_DISABLE_COMPOSITING_MODE: &str = "WEBKIT_DISABLE_COMPOSITING_MODE";
const NV_DISABLE_EXPLICIT_SYNC: &str = "__NV_DISABLE_EXPLICIT_SYNC";
/// Present while NVIDIA's proprietary kernel driver is loaded.
const NVIDIA_DRIVER_VERSION_FILE: &str = "/proc/driver/nvidia/version";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RenderingMode {
    /// Software compositing: slower, but known not to crash or blank the
    /// window on NVIDIA and KDE Plasma Wayland.
    #[default]
    Safe,
    /// WebKitGTK's GPU compositing and DMABUF presentation.
    Gpu,
}

impl RenderingMode {
    pub fn as_str(self) -> &'static str {
        match self {
            RenderingMode::Safe => "safe",
            RenderingMode::Gpu => "gpu",
        }
    }

    /// `gpu` or `safe`, ignoring case and surrounding space.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "safe" => Some(RenderingMode::Safe),
            "gpu" => Some(RenderingMode::Gpu),
            _ => None,
        }
    }

    /// The choice a preference file holds. Empty content is no choice;
    /// content that is neither word is `Safe`.
    pub fn from_file_content(content: &str) -> Option<Self> {
        if content.trim().is_empty() {
            return None;
        }
        Some(Self::parse(content).unwrap_or_default())
    }

    pub fn file_content(self) -> String {
        format!("{}\n", self.as_str())
    }
}

/// What a start does about rendering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderingDecision {
    /// False where the setting has no effect (not Linux, or not Wayland);
    /// nothing is set then.
    pub applies: bool,
    /// The mode this start uses. `Gpu` when nothing chose a mode, because
    /// nothing is turned off then.
    pub mode: RenderingMode,
    /// `SYNARA_RENDERING` decided the mode instead of the saved setting.
    pub overridden_by_environment: bool,
    /// Variables to set before GTK initialises.
    pub set: Vec<(&'static str, &'static str)>,
}

/// A non-empty value of one of the two mode variables: `gpu` is GPU, anything
/// else is safe (as the launcher script treated `SYNARA_RENDERING` before
/// this moved here).
fn mode_from_environment(
    lookup: &impl Fn(&str) -> Option<String>,
    name: &str,
) -> Option<RenderingMode> {
    lookup(name)
        .filter(|value| !value.trim().is_empty())
        .map(|value| RenderingMode::parse(&value).unwrap_or_default())
}

/// Decide the mode and the variables to set. Pure, so tests need no process
/// environment. `os` is `std::env::consts::OS`; `saved` is `None` while the
/// user has not chosen.
///
/// The first of these that names a mode decides: `SYNARA_RENDERING`, the
/// saved setting, `SYNARA_RENDERING_DEFAULT` (the package's default). When
/// none does, nothing is set, as in a build started without a launcher.
///
/// - Safe sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` and
///   `WEBKIT_DISABLE_COMPOSITING_MODE=1`.
/// - A chosen GPU sets nothing for WebKit. With NVIDIA's driver it sets
///   `__NV_DISABLE_EXPLICIT_SYNC=1`: GTK 3 commits a buffer without an
///   acquire point and the compositor ends the connection ("Missing acquire
///   timeline", Error 71).
/// - A variable already in the environment, even an empty one, is left alone.
pub fn decide(
    os: &str,
    is_wayland: bool,
    saved: Option<RenderingMode>,
    lookup: impl Fn(&str) -> Option<String>,
    nvidia_driver_present: bool,
) -> RenderingDecision {
    let requested = mode_from_environment(&lookup, RENDERING_ENV);
    let overridden_by_environment = requested.is_some();
    let chosen = requested
        .or(saved)
        .or_else(|| mode_from_environment(&lookup, RENDERING_DEFAULT_ENV));
    let applies = os == "linux" && is_wayland;
    let wanted: &[&'static str] = match (applies, chosen) {
        (false, _) | (true, None) => &[],
        (true, Some(RenderingMode::Safe)) => &[
            WEBKIT_DISABLE_DMABUF_RENDERER,
            WEBKIT_DISABLE_COMPOSITING_MODE,
        ],
        (true, Some(RenderingMode::Gpu)) if nvidia_driver_present => &[NV_DISABLE_EXPLICIT_SYNC],
        (true, Some(RenderingMode::Gpu)) => &[],
    };
    let set = wanted
        .iter()
        .filter(|name| lookup(name).is_none())
        .map(|name| (*name, "1"))
        .collect();
    RenderingDecision {
        applies,
        mode: chosen.unwrap_or(RenderingMode::Gpu),
        overridden_by_environment,
        set,
    }
}

pub fn nvidia_driver_present() -> bool {
    Path::new(NVIDIA_DRIVER_VERSION_FILE).exists()
}

/// `<config dir>/<app identifier>/rendering-mode`, the same directory Tauri's
/// `app_config_dir` resolves to, computed without a running app.
pub fn preference_path(config_dir: Option<PathBuf>, identifier: &str) -> Option<PathBuf> {
    if identifier.is_empty() {
        return None;
    }
    config_dir.map(|dir| dir.join(identifier).join(PREFERENCE_FILE_NAME))
}

/// The saved choice; `None` when the file is missing, unreadable or empty.
pub fn read_saved_mode(path: &Path) -> Option<RenderingMode> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|content| RenderingMode::from_file_content(&content))
}

pub fn write_saved_mode(path: &Path, mode: RenderingMode) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, mode.file_content())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn decide_with(
        os: &str,
        is_wayland: bool,
        saved: Option<RenderingMode>,
        vars: &[(&str, &str)],
        nvidia: bool,
    ) -> RenderingDecision {
        let map: HashMap<String, String> = vars
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect();
        decide(os, is_wayland, saved, |key| map.get(key).cloned(), nvidia)
    }

    const SAFE: Option<RenderingMode> = Some(RenderingMode::Safe);
    const GPU: Option<RenderingMode> = Some(RenderingMode::Gpu);
    const SAFE_VARIABLES: [(&str, &str); 2] = [
        ("WEBKIT_DISABLE_DMABUF_RENDERER", "1"),
        ("WEBKIT_DISABLE_COMPOSITING_MODE", "1"),
    ];
    const NVIDIA_VARIABLES: [(&str, &str); 1] = [("__NV_DISABLE_EXPLICIT_SYNC", "1")];

    #[test]
    fn with_nothing_chosen_and_no_package_default_nothing_is_set() {
        for nvidia in [false, true] {
            let decision = decide_with("linux", true, None, &[], nvidia);
            assert!(decision.applies);
            assert_eq!(decision.mode, RenderingMode::Gpu);
            assert!(!decision.overridden_by_environment);
            assert!(decision.set.is_empty());
        }
    }

    #[test]
    fn the_package_default_applies_while_nothing_is_chosen() {
        let safe = decide_with(
            "linux",
            true,
            None,
            &[("SYNARA_RENDERING_DEFAULT", "safe")],
            true,
        );
        assert_eq!(safe.mode, RenderingMode::Safe);
        assert!(!safe.overridden_by_environment);
        assert_eq!(safe.set, SAFE_VARIABLES);

        let gpu = decide_with(
            "linux",
            true,
            None,
            &[("SYNARA_RENDERING_DEFAULT", "gpu")],
            true,
        );
        assert_eq!(gpu.mode, RenderingMode::Gpu);
        assert_eq!(gpu.set, NVIDIA_VARIABLES);

        let unknown = decide_with(
            "linux",
            true,
            None,
            &[("SYNARA_RENDERING_DEFAULT", "cpu")],
            false,
        );
        assert_eq!(unknown.mode, RenderingMode::Safe);

        let empty = decide_with(
            "linux",
            true,
            None,
            &[("SYNARA_RENDERING_DEFAULT", " ")],
            true,
        );
        assert_eq!(empty.mode, RenderingMode::Gpu);
        assert!(empty.set.is_empty());
    }

    #[test]
    fn the_saved_setting_wins_over_the_package_default() {
        let gpu = decide_with(
            "linux",
            true,
            GPU,
            &[("SYNARA_RENDERING_DEFAULT", "safe")],
            false,
        );
        assert_eq!(gpu.mode, RenderingMode::Gpu);
        assert!(!gpu.overridden_by_environment);
        assert!(gpu.set.is_empty());

        let safe = decide_with(
            "linux",
            true,
            SAFE,
            &[("SYNARA_RENDERING_DEFAULT", "gpu")],
            true,
        );
        assert_eq!(safe.mode, RenderingMode::Safe);
        assert_eq!(safe.set, SAFE_VARIABLES);
    }

    #[test]
    fn saved_safe_turns_gpu_drawing_off_without_a_package_default() {
        let decision = decide_with("linux", true, SAFE, &[], true);
        assert_eq!(decision.mode, RenderingMode::Safe);
        assert_eq!(decision.set, SAFE_VARIABLES);
    }

    #[test]
    fn saved_gpu_sets_nothing_without_nvidia() {
        let decision = decide_with("linux", true, GPU, &[], false);
        assert_eq!(decision.mode, RenderingMode::Gpu);
        assert!(decision.set.is_empty());
    }

    #[test]
    fn saved_gpu_turns_explicit_sync_off_on_nvidia() {
        let decision = decide_with("linux", true, GPU, &[], true);
        assert_eq!(decision.set, NVIDIA_VARIABLES);
    }

    #[test]
    fn the_environment_variable_wins_over_the_saved_setting_and_the_default() {
        let gpu = decide_with(
            "linux",
            true,
            SAFE,
            &[
                ("SYNARA_RENDERING", " GPU "),
                ("SYNARA_RENDERING_DEFAULT", "safe"),
            ],
            true,
        );
        assert_eq!(gpu.mode, RenderingMode::Gpu);
        assert!(gpu.overridden_by_environment);
        assert_eq!(gpu.set, NVIDIA_VARIABLES);

        let safe = decide_with("linux", true, GPU, &[("SYNARA_RENDERING", "safe")], true);
        assert_eq!(safe.mode, RenderingMode::Safe);
        assert!(safe.overridden_by_environment);
        assert_eq!(safe.set, SAFE_VARIABLES);
    }

    #[test]
    fn an_unknown_environment_value_means_safe() {
        let decision = decide_with(
            "linux",
            true,
            GPU,
            &[("SYNARA_RENDERING", "software")],
            false,
        );
        assert_eq!(decision.mode, RenderingMode::Safe);
        assert!(decision.overridden_by_environment);
        assert_eq!(decision.set, SAFE_VARIABLES);
    }

    #[test]
    fn an_empty_environment_value_leaves_the_saved_setting_in_charge() {
        let decision = decide_with("linux", true, GPU, &[("SYNARA_RENDERING", " ")], false);
        assert_eq!(decision.mode, RenderingMode::Gpu);
        assert!(!decision.overridden_by_environment);
    }

    #[test]
    fn variables_already_in_the_environment_are_never_overwritten() {
        let safe = decide_with(
            "linux",
            true,
            SAFE,
            &[("WEBKIT_DISABLE_COMPOSITING_MODE", "0")],
            false,
        );
        assert_eq!(safe.set, [("WEBKIT_DISABLE_DMABUF_RENDERER", "1")]);

        let empty = decide_with(
            "linux",
            true,
            SAFE,
            &[
                ("WEBKIT_DISABLE_DMABUF_RENDERER", ""),
                ("WEBKIT_DISABLE_COMPOSITING_MODE", "1"),
            ],
            false,
        );
        assert!(empty.set.is_empty());

        let gpu = decide_with(
            "linux",
            true,
            GPU,
            &[("__NV_DISABLE_EXPLICIT_SYNC", "0")],
            true,
        );
        assert!(gpu.set.is_empty());
    }

    #[test]
    fn nothing_is_set_outside_linux_wayland() {
        for (os, is_wayland) in [("linux", false), ("macos", true), ("windows", true)] {
            for saved in [None, SAFE, GPU] {
                let decision = decide_with(
                    os,
                    is_wayland,
                    saved,
                    &[("SYNARA_RENDERING_DEFAULT", "safe")],
                    true,
                );
                assert!(!decision.applies, "{os}");
                assert!(decision.set.is_empty(), "{os}");
            }
        }
    }

    #[test]
    fn file_content_parses_gpu_and_safe() {
        assert_eq!(RenderingMode::from_file_content("gpu\n"), GPU);
        assert_eq!(RenderingMode::from_file_content("  GPU  "), GPU);
        assert_eq!(RenderingMode::from_file_content("safe\n"), SAFE);
    }

    #[test]
    fn empty_file_content_is_no_choice() {
        for content in ["", "\n", "  \t\n"] {
            assert_eq!(RenderingMode::from_file_content(content), None);
        }
    }

    #[test]
    fn unknown_file_content_is_safe() {
        for content in ["on", "true", "gpu safe", "{\"mode\":\"gpu\"}"] {
            assert_eq!(
                RenderingMode::from_file_content(content),
                SAFE,
                "{content:?}"
            );
        }
    }

    #[test]
    fn file_content_round_trips() {
        for mode in [RenderingMode::Safe, RenderingMode::Gpu] {
            assert_eq!(
                RenderingMode::from_file_content(&mode.file_content()),
                Some(mode)
            );
        }
    }

    #[test]
    fn the_mode_serialises_as_a_lower_case_word() {
        assert_eq!(
            serde_json::to_string(&RenderingMode::Gpu).unwrap(),
            "\"gpu\""
        );
        assert_eq!(
            serde_json::from_str::<RenderingMode>("\"safe\"").unwrap(),
            RenderingMode::Safe
        );
        assert!(serde_json::from_str::<RenderingMode>("\"fast\"").is_err());
    }

    #[test]
    fn the_preference_file_sits_in_the_app_config_directory() {
        assert_eq!(
            preference_path(Some(PathBuf::from("/home/u/.config")), "com.example.app"),
            Some(PathBuf::from(
                "/home/u/.config/com.example.app/rendering-mode"
            ))
        );
        assert_eq!(preference_path(None, "com.example.app"), None);
        assert_eq!(
            preference_path(Some(PathBuf::from("/home/u/.config")), ""),
            None
        );
    }

    #[test]
    fn a_saved_mode_is_read_back_and_a_missing_file_is_no_choice() {
        let dir =
            std::env::temp_dir().join(format!("synara-rendering-mode-test-{}", std::process::id()));
        let path = dir.join("nested").join(PREFERENCE_FILE_NAME);
        assert_eq!(read_saved_mode(&path), None);
        write_saved_mode(&path, RenderingMode::Gpu).unwrap();
        assert_eq!(read_saved_mode(&path), GPU);
        write_saved_mode(&path, RenderingMode::Safe).unwrap();
        assert_eq!(read_saved_mode(&path), SAFE);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
