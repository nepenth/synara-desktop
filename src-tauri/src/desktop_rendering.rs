//! How the Linux web view draws: entirely on the CPU ("safe"), composited on
//! the graphics card but presented without DMABUF buffers ("balanced"), or
//! fully on the graphics card ("gpu"). It is the "Hardware acceleration"
//! choice in Settings.
//!
//! GTK and WebKitGTK read the choice from environment variables when the
//! process starts, so it is decided before the app builds its window and a
//! change applies on the next start. The saved choice is a one-word file,
//! `rendering-mode`, in the app's config directory; no file means the user
//! has not chosen, and the package's default applies. macOS and Windows web
//! views are always GPU-accelerated and never read it.
//!
//! When the web view fails in Balanced or Full, the app falls back to safe on
//! its own. A web process that crashes while the app runs, and a start that
//! ends before the interface has drawn (NVIDIA's "Error 71" takes the whole
//! process down), both leave a `rendering-fallback` file naming the mode that
//! failed. Later starts draw safely while it exists; choosing a mode in
//! Settings removes it. The saved choice itself is never rewritten. A start
//! that may fail leaves a `rendering-startup` file, removed once the interface
//! has drawn or the app exits normally; the next start finding it knows the
//! last one died.
//!
//! This module has no Tauri types; `desktop_rendering_host` holds the commands,
//! the startup call and the web process watch.

use serde::{Deserialize, Serialize};
#[cfg(any(target_os = "linux", test))]
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Environment variable that picks the mode for one start and wins over the
/// saved setting.
pub const RENDERING_ENV: &str = "SYNARA_RENDERING";
/// Environment variable a package's launcher sets to say what applies while
/// the user has not chosen. Without it nothing is changed.
pub const RENDERING_DEFAULT_ENV: &str = "SYNARA_RENDERING_DEFAULT";
/// Name of the preference file in the app's config directory.
pub const PREFERENCE_FILE_NAME: &str = "rendering-mode";
/// Present while starts fall back to safe after a failure.
pub const FALLBACK_FILE_NAME: &str = "rendering-fallback";
/// Present while a start that may fail has not yet drawn its interface.
pub const STARTUP_SENTINEL_FILE_NAME: &str = "rendering-startup";

const WEBKIT_DISABLE_DMABUF_RENDERER: &str = "WEBKIT_DISABLE_DMABUF_RENDERER";
const WEBKIT_DISABLE_COMPOSITING_MODE: &str = "WEBKIT_DISABLE_COMPOSITING_MODE";
const NV_DISABLE_EXPLICIT_SYNC: &str = "__NV_DISABLE_EXPLICIT_SYNC";
/// Present while NVIDIA's proprietary kernel driver is loaded.
const NVIDIA_DRIVER_VERSION_FILE: &str = "/proc/driver/nvidia/version";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RenderingMode {
    /// Software compositing: slower, but known not to crash or blank the
    /// window on NVIDIA and KDE Plasma Wayland. Every animation frame is
    /// painted on the CPU and copied on the UI process's main thread, so a
    /// running animation keeps that thread busy.
    #[default]
    Safe,
    /// WebKitGTK's GPU compositing without the DMABUF renderer, the path
    /// behind most NVIDIA blank-window and crash reports. Animations are
    /// composited on the graphics card instead of repainted on the CPU.
    Balanced,
    /// WebKitGTK's GPU compositing and DMABUF presentation.
    Gpu,
}

impl RenderingMode {
    pub fn as_str(self) -> &'static str {
        match self {
            RenderingMode::Safe => "safe",
            RenderingMode::Balanced => "balanced",
            RenderingMode::Gpu => "gpu",
        }
    }

    /// `gpu`, `balanced` or `safe`, ignoring case and surrounding space.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "safe" => Some(RenderingMode::Safe),
            "balanced" => Some(RenderingMode::Balanced),
            "gpu" => Some(RenderingMode::Gpu),
            _ => None,
        }
    }

    /// The choice a preference file holds. Empty content is no choice;
    /// content that is none of the words is `Safe`.
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

/// Why the app stopped using a mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FallbackReason {
    /// The web view's process crashed while the app was running.
    WebViewCrashed,
    /// The app ended before its interface had drawn.
    StartFailed,
}

/// The mode that failed, and how. While it is recorded, starts draw safely.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderingFallback {
    pub mode: RenderingMode,
    pub reason: FallbackReason,
}

impl RenderingFallback {
    /// `None` for empty or unreadable content: only this module writes the
    /// file, so anything else is not a fallback it recorded.
    pub fn from_file_content(content: &str) -> Option<Self> {
        serde_json::from_str(content.trim()).ok()
    }

    pub fn file_content(self) -> String {
        let json = serde_json::to_string(&self).unwrap_or_default();
        format!("{json}\n")
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
    /// The recorded failure that made this start safe instead of the mode the
    /// setting or the package's default names.
    pub fallback: Option<RenderingFallback>,
    /// Variables to set before GTK initialises.
    pub set: Vec<(&'static str, &'static str)>,
}

/// A non-empty value of one of the two mode variables: `gpu` and `balanced`
/// name those modes, anything else is safe (as the launcher script treated `SYNARA_RENDERING` before
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
/// user has not chosen; `fallback` is the recorded failure, if any.
///
/// The first of these that names a mode decides: `SYNARA_RENDERING`, the
/// saved setting, `SYNARA_RENDERING_DEFAULT` (the package's default). When
/// none does, nothing is set, as in a build started without a launcher.
/// A recorded fallback makes the start safe unless `SYNARA_RENDERING`
/// decides or the setting or default already is safe; "nothing chosen" falls
/// back too, since it draws like Full.
///
/// - Safe sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` and
///   `WEBKIT_DISABLE_COMPOSITING_MODE=1`.
/// - Balanced sets only `WEBKIT_DISABLE_DMABUF_RENDERER=1`, keeping GPU
///   compositing.
/// - A chosen GPU sets nothing for WebKit.
/// - Balanced and GPU with NVIDIA's driver also set
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
    fallback: Option<RenderingFallback>,
) -> RenderingDecision {
    let requested = mode_from_environment(&lookup, RENDERING_ENV);
    let overridden_by_environment = requested.is_some();
    let preferred = saved.or_else(|| mode_from_environment(&lookup, RENDERING_DEFAULT_ENV));
    let applies = os == "linux" && is_wayland;
    let fallback = fallback.filter(|_| {
        applies && !overridden_by_environment && preferred != Some(RenderingMode::Safe)
    });
    let chosen = if fallback.is_some() {
        Some(RenderingMode::Safe)
    } else {
        requested.or(preferred)
    };
    let wanted: &[&'static str] = match (applies, chosen) {
        (false, _) | (true, None) => &[],
        (true, Some(RenderingMode::Safe)) => &[
            WEBKIT_DISABLE_DMABUF_RENDERER,
            WEBKIT_DISABLE_COMPOSITING_MODE,
        ],
        (true, Some(RenderingMode::Balanced)) if nvidia_driver_present => {
            &[WEBKIT_DISABLE_DMABUF_RENDERER, NV_DISABLE_EXPLICIT_SYNC]
        }
        (true, Some(RenderingMode::Balanced)) => &[WEBKIT_DISABLE_DMABUF_RENDERER],
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
        fallback,
        set,
    }
}

/// The note a start that may fail leaves until its interface has drawn: the
/// mode and the process id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StartupSentinel {
    pub mode: RenderingMode,
    pub pid: u32,
}

impl StartupSentinel {
    /// The sentinel this start leaves, if it can fail in a way a safe start
    /// would not: a mode other than safe that the setting, the package's
    /// default or nothing picked. A mode forced with `SYNARA_RENDERING` is the
    /// user's to keep, so it never falls back and needs no sentinel.
    pub fn for_start(decision: &RenderingDecision, pid: u32) -> Option<Self> {
        (decision.applies
            && !decision.overridden_by_environment
            && decision.mode != RenderingMode::Safe)
            .then_some(StartupSentinel {
                mode: decision.mode,
                pid,
            })
    }

    pub fn file_content(self) -> String {
        format!("{} {}\n", self.mode.as_str(), self.pid)
    }

    /// `<mode> <pid>`; `None` for anything else.
    pub fn from_file_content(content: &str) -> Option<Self> {
        let mut words = content.split_whitespace();
        let mode = RenderingMode::parse(words.next()?)?;
        let pid = words.next()?.parse().ok()?;
        words
            .next()
            .is_none()
            .then_some(StartupSentinel { mode, pid })
    }
}

/// What the sentinel file says about the previous start.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviousStart {
    /// No sentinel: it drew its interface or exited normally.
    Finished,
    /// Another copy of the app is still starting (a second launch while the
    /// first is running hands over to it and exits). Its sentinel stays.
    StillStarting,
    /// Content this module did not write. The file is removed.
    Unreadable,
    /// It ended before drawing its interface, in this mode. The file is
    /// removed.
    Failed(RenderingMode),
}

/// Pure: `content` is the sentinel file's content, `None` when it is missing;
/// `is_running` says whether a process id is a running copy of the app.
pub fn previous_start(
    content: Option<&str>,
    own_pid: u32,
    is_running: impl Fn(u32) -> bool,
) -> PreviousStart {
    let Some(content) = content else {
        return PreviousStart::Finished;
    };
    match StartupSentinel::from_file_content(content) {
        None => PreviousStart::Unreadable,
        Some(sentinel) if sentinel.pid != own_pid && is_running(sentinel.pid) => {
            PreviousStart::StillStarting
        }
        Some(sentinel) if sentinel.mode == RenderingMode::Safe => PreviousStart::Unreadable,
        Some(sentinel) => PreviousStart::Failed(sentinel.mode),
    }
}

/// The fallback to record after the previous start, if there is a new one.
/// One already recorded is kept: it names the mode that failed first.
pub fn fallback_after(
    previous: PreviousStart,
    recorded: Option<RenderingFallback>,
) -> Option<RenderingFallback> {
    match (previous, recorded) {
        (PreviousStart::Failed(mode), None) => Some(RenderingFallback {
            mode,
            reason: FallbackReason::StartFailed,
        }),
        _ => None,
    }
}

/// How WebKitGTK's web process ended (`WebProcessTerminationReason`).
#[cfg(any(target_os = "linux", test))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebProcessExit {
    Crashed,
    ExceededMemoryLimit,
    /// The app asked for it; not a failure.
    TerminatedByApi,
    Unknown,
}

/// What to do when the web process ends.
#[cfg(any(target_os = "linux", test))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebProcessExitResponse {
    Ignore,
    /// Safe drawing would not avoid it, or the mode is the user's to keep:
    /// only note it in the log.
    Log,
    /// Record this fallback and start again, safely.
    FallBack(RenderingFallback),
}

/// Pure. Only a crash falls back: a memory-limit kill is not a drawing
/// failure, and a mode forced with `SYNARA_RENDERING` is never overridden.
#[cfg(any(target_os = "linux", test))]
pub fn web_process_exit_response(
    decision: &RenderingDecision,
    exit: WebProcessExit,
) -> WebProcessExitResponse {
    match exit {
        WebProcessExit::TerminatedByApi => WebProcessExitResponse::Ignore,
        WebProcessExit::Crashed
            if decision.applies
                && !decision.overridden_by_environment
                && decision.mode != RenderingMode::Safe =>
        {
            WebProcessExitResponse::FallBack(RenderingFallback {
                mode: decision.mode,
                reason: FallbackReason::WebViewCrashed,
            })
        }
        _ => WebProcessExitResponse::Log,
    }
}

/// The environment for a copy of the app started by this one: this process's
/// environment without the variables it set itself at startup, so the copy
/// decides again. Everything else, the launcher's variables included, stays.
#[cfg(any(target_os = "linux", test))]
pub fn relaunch_environment(
    current: impl IntoIterator<Item = (OsString, OsString)>,
    set_at_startup: &[(&'static str, &'static str)],
) -> Vec<(OsString, OsString)> {
    current
        .into_iter()
        .filter(|(name, _)| !set_at_startup.iter().any(|(set, _)| name == set))
        .collect()
}

pub fn nvidia_driver_present() -> bool {
    Path::new(NVIDIA_DRIVER_VERSION_FILE).exists()
}

/// `<config dir>/<app identifier>`, the directory Tauri's `app_config_dir`
/// resolves to, computed without a running app.
pub fn app_config_dir(config_dir: Option<PathBuf>, identifier: &str) -> Option<PathBuf> {
    if identifier.is_empty() {
        return None;
    }
    config_dir.map(|dir| dir.join(identifier))
}

/// The saved choice; `None` when the file is missing, unreadable or empty.
pub fn read_saved_mode(path: &Path) -> Option<RenderingMode> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|content| RenderingMode::from_file_content(&content))
}

pub fn write_saved_mode(path: &Path, mode: RenderingMode) -> std::io::Result<()> {
    write_creating_parent(path, &mode.file_content())
}

/// The recorded fallback; `None` when the file is missing or unreadable.
pub fn read_fallback(path: &Path) -> Option<RenderingFallback> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|content| RenderingFallback::from_file_content(&content))
}

pub fn write_fallback(path: &Path, fallback: RenderingFallback) -> std::io::Result<()> {
    write_creating_parent(path, &fallback.file_content())
}

/// Removes a file; one that is already gone is not an error.
pub fn remove_if_present(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

pub fn write_creating_parent(path: &Path, content: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)
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
        decide(
            os,
            is_wayland,
            saved,
            |key| map.get(key).cloned(),
            nvidia,
            None,
        )
    }

    fn decide_after_failure(
        saved: Option<RenderingMode>,
        vars: &[(&str, &str)],
        fallback: RenderingFallback,
    ) -> RenderingDecision {
        let map: HashMap<String, String> = vars
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect();
        decide(
            "linux",
            true,
            saved,
            |key| map.get(key).cloned(),
            true,
            Some(fallback),
        )
    }

    const BALANCED_CRASHED: RenderingFallback = RenderingFallback {
        mode: RenderingMode::Balanced,
        reason: FallbackReason::WebViewCrashed,
    };
    const GPU_START_FAILED: RenderingFallback = RenderingFallback {
        mode: RenderingMode::Gpu,
        reason: FallbackReason::StartFailed,
    };

    const SAFE: Option<RenderingMode> = Some(RenderingMode::Safe);
    const GPU: Option<RenderingMode> = Some(RenderingMode::Gpu);
    const BALANCED: Option<RenderingMode> = Some(RenderingMode::Balanced);
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
    fn balanced_keeps_compositing_and_turns_only_dmabuf_off() {
        let decision = decide_with("linux", true, BALANCED, &[], false);
        assert_eq!(decision.mode, RenderingMode::Balanced);
        assert_eq!(decision.set, [("WEBKIT_DISABLE_DMABUF_RENDERER", "1")]);
    }

    #[test]
    fn balanced_turns_explicit_sync_off_on_nvidia() {
        let decision = decide_with("linux", true, BALANCED, &[], true);
        assert_eq!(
            decision.set,
            [
                ("WEBKIT_DISABLE_DMABUF_RENDERER", "1"),
                ("__NV_DISABLE_EXPLICIT_SYNC", "1"),
            ]
        );
    }

    #[test]
    fn balanced_can_be_the_package_default_or_the_environment_choice() {
        let default = decide_with(
            "linux",
            true,
            None,
            &[("SYNARA_RENDERING_DEFAULT", "balanced")],
            false,
        );
        assert_eq!(default.mode, RenderingMode::Balanced);
        assert!(!default.overridden_by_environment);

        let env = decide_with(
            "linux",
            true,
            SAFE,
            &[("SYNARA_RENDERING", " Balanced ")],
            false,
        );
        assert_eq!(env.mode, RenderingMode::Balanced);
        assert!(env.overridden_by_environment);
        assert_eq!(env.set, [("WEBKIT_DISABLE_DMABUF_RENDERER", "1")]);
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
            for saved in [None, SAFE, BALANCED, GPU] {
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
    fn file_content_parses_every_mode() {
        assert_eq!(RenderingMode::from_file_content("gpu\n"), GPU);
        assert_eq!(RenderingMode::from_file_content("  GPU  "), GPU);
        assert_eq!(RenderingMode::from_file_content("safe\n"), SAFE);
        assert_eq!(RenderingMode::from_file_content("balanced\n"), BALANCED);
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
        for mode in [
            RenderingMode::Safe,
            RenderingMode::Balanced,
            RenderingMode::Gpu,
        ] {
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
        assert_eq!(
            serde_json::to_string(&RenderingMode::Balanced).unwrap(),
            "\"balanced\""
        );
        assert!(serde_json::from_str::<RenderingMode>("\"fast\"").is_err());
    }

    #[test]
    fn the_files_sit_in_the_app_config_directory() {
        assert_eq!(
            app_config_dir(Some(PathBuf::from("/home/u/.config")), "com.example.app"),
            Some(PathBuf::from("/home/u/.config/com.example.app"))
        );
        assert_eq!(app_config_dir(None, "com.example.app"), None);
        assert_eq!(
            app_config_dir(Some(PathBuf::from("/home/u/.config")), ""),
            None
        );
    }

    #[test]
    fn a_fallback_makes_a_balanced_or_full_start_safe() {
        for saved in [BALANCED, GPU] {
            let decision = decide_after_failure(saved, &[], BALANCED_CRASHED);
            assert_eq!(decision.mode, RenderingMode::Safe, "{saved:?}");
            assert_eq!(decision.fallback, Some(BALANCED_CRASHED));
            assert!(!decision.overridden_by_environment);
            assert_eq!(decision.set, SAFE_VARIABLES);
        }
    }

    #[test]
    fn a_fallback_overrides_the_package_default_and_no_choice_at_all() {
        let default = decide_after_failure(
            None,
            &[("SYNARA_RENDERING_DEFAULT", "balanced")],
            GPU_START_FAILED,
        );
        assert_eq!(default.mode, RenderingMode::Safe);
        assert_eq!(default.fallback, Some(GPU_START_FAILED));
        assert_eq!(default.set, SAFE_VARIABLES);

        let nothing = decide_after_failure(None, &[], GPU_START_FAILED);
        assert_eq!(nothing.mode, RenderingMode::Safe);
        assert_eq!(nothing.fallback, Some(GPU_START_FAILED));
        assert_eq!(nothing.set, SAFE_VARIABLES);
    }

    #[test]
    fn the_saved_setting_wins_over_a_fallback_default_and_safe_needs_no_fallback() {
        let saved_safe = decide_after_failure(
            SAFE,
            &[("SYNARA_RENDERING_DEFAULT", "balanced")],
            BALANCED_CRASHED,
        );
        assert_eq!(saved_safe.mode, RenderingMode::Safe);
        assert_eq!(saved_safe.fallback, None);

        let default_safe = decide_after_failure(
            None,
            &[("SYNARA_RENDERING_DEFAULT", "safe")],
            BALANCED_CRASHED,
        );
        assert_eq!(default_safe.fallback, None);
    }

    #[test]
    fn the_environment_variable_wins_over_a_fallback() {
        let decision =
            decide_after_failure(BALANCED, &[("SYNARA_RENDERING", "gpu")], BALANCED_CRASHED);
        assert_eq!(decision.mode, RenderingMode::Gpu);
        assert!(decision.overridden_by_environment);
        assert_eq!(decision.fallback, None);
        assert_eq!(decision.set, NVIDIA_VARIABLES);
    }

    #[test]
    fn a_fallback_means_nothing_outside_linux_wayland() {
        for (os, is_wayland) in [("linux", false), ("macos", true)] {
            let decision = decide(os, is_wayland, GPU, |_| None, false, Some(BALANCED_CRASHED));
            assert!(!decision.applies);
            assert_eq!(decision.fallback, None);
            assert!(decision.set.is_empty());
        }
    }

    #[test]
    fn a_fallback_still_leaves_variables_already_in_the_environment_alone() {
        let decision = decide_after_failure(
            GPU,
            &[("WEBKIT_DISABLE_COMPOSITING_MODE", "0")],
            BALANCED_CRASHED,
        );
        assert_eq!(decision.mode, RenderingMode::Safe);
        assert_eq!(decision.set, [("WEBKIT_DISABLE_DMABUF_RENDERER", "1")]);
    }

    #[test]
    fn fallback_file_content_round_trips_and_reads_as_json() {
        for fallback in [BALANCED_CRASHED, GPU_START_FAILED] {
            assert_eq!(
                RenderingFallback::from_file_content(&fallback.file_content()),
                Some(fallback)
            );
        }
        assert_eq!(
            BALANCED_CRASHED.file_content(),
            "{\"mode\":\"balanced\",\"reason\":\"web-view-crashed\"}\n"
        );
        assert_eq!(
            GPU_START_FAILED.file_content(),
            "{\"mode\":\"gpu\",\"reason\":\"start-failed\"}\n"
        );
    }

    #[test]
    fn unreadable_fallback_content_is_no_fallback() {
        for content in [
            "",
            "\n",
            "balanced",
            "{\"mode\":\"balanced\"}",
            "{\"mode\":\"fast\",\"reason\":\"start-failed\"}",
            "{\"mode\":\"gpu\",\"reason\":\"bored\"}",
        ] {
            assert_eq!(
                RenderingFallback::from_file_content(content),
                None,
                "{content:?}"
            );
        }
    }

    fn decision_for(mode: RenderingMode, applies: bool, overridden: bool) -> RenderingDecision {
        RenderingDecision {
            applies,
            mode,
            overridden_by_environment: overridden,
            fallback: None,
            set: Vec::new(),
        }
    }

    #[test]
    fn only_a_start_that_may_fail_leaves_a_sentinel() {
        for mode in [RenderingMode::Balanced, RenderingMode::Gpu] {
            assert_eq!(
                StartupSentinel::for_start(&decision_for(mode, true, false), 42),
                Some(StartupSentinel { mode, pid: 42 })
            );
            assert_eq!(
                StartupSentinel::for_start(&decision_for(mode, true, true), 42),
                None,
                "forced with SYNARA_RENDERING"
            );
            assert_eq!(
                StartupSentinel::for_start(&decision_for(mode, false, false), 42),
                None,
                "not Linux Wayland"
            );
        }
        assert_eq!(
            StartupSentinel::for_start(&decision_for(RenderingMode::Safe, true, false), 42),
            None
        );
    }

    #[test]
    fn sentinel_content_round_trips_and_rejects_anything_else() {
        let sentinel = StartupSentinel {
            mode: RenderingMode::Balanced,
            pid: 4242,
        };
        assert_eq!(sentinel.file_content(), "balanced 4242\n");
        assert_eq!(
            StartupSentinel::from_file_content(&sentinel.file_content()),
            Some(sentinel)
        );
        for content in [
            "",
            "balanced",
            "4242",
            "fast 4242",
            "gpu -1",
            "gpu 12 13",
            "gpu x",
        ] {
            assert_eq!(
                StartupSentinel::from_file_content(content),
                None,
                "{content:?}"
            );
        }
    }

    #[test]
    fn no_sentinel_means_the_previous_start_finished() {
        assert_eq!(previous_start(None, 7, |_| true), PreviousStart::Finished);
    }

    #[test]
    fn a_sentinel_from_a_process_that_is_gone_is_a_failed_start() {
        assert_eq!(
            previous_start(Some("balanced 100\n"), 7, |_| false),
            PreviousStart::Failed(RenderingMode::Balanced)
        );
        assert_eq!(
            previous_start(Some("gpu 100"), 7, |pid| pid != 100),
            PreviousStart::Failed(RenderingMode::Gpu)
        );
    }

    #[test]
    fn a_sentinel_from_a_copy_that_is_still_starting_is_left_alone() {
        assert_eq!(
            previous_start(Some("balanced 100\n"), 7, |pid| pid == 100),
            PreviousStart::StillStarting
        );
    }

    #[test]
    fn a_sentinel_with_this_process_id_is_from_an_earlier_process() {
        assert_eq!(
            previous_start(Some("gpu 7\n"), 7, |_| true),
            PreviousStart::Failed(RenderingMode::Gpu)
        );
    }

    #[test]
    fn an_unreadable_or_safe_sentinel_is_not_a_failure() {
        for content in ["", "garbage", "safe 100"] {
            assert_eq!(
                previous_start(Some(content), 7, |_| false),
                PreviousStart::Unreadable,
                "{content:?}"
            );
        }
    }

    #[test]
    fn a_failed_start_records_a_fallback_unless_one_is_recorded() {
        assert_eq!(
            fallback_after(PreviousStart::Failed(RenderingMode::Gpu), None),
            Some(GPU_START_FAILED)
        );
        assert_eq!(
            fallback_after(
                PreviousStart::Failed(RenderingMode::Gpu),
                Some(BALANCED_CRASHED)
            ),
            None
        );
        for previous in [
            PreviousStart::Finished,
            PreviousStart::StillStarting,
            PreviousStart::Unreadable,
        ] {
            assert_eq!(fallback_after(previous, None), None, "{previous:?}");
        }
    }

    #[test]
    fn a_web_process_crash_in_balanced_or_full_falls_back() {
        for mode in [RenderingMode::Balanced, RenderingMode::Gpu] {
            assert_eq!(
                web_process_exit_response(
                    &decision_for(mode, true, false),
                    WebProcessExit::Crashed
                ),
                WebProcessExitResponse::FallBack(RenderingFallback {
                    mode,
                    reason: FallbackReason::WebViewCrashed,
                })
            );
        }
    }

    #[test]
    fn a_web_process_ended_by_the_app_is_not_a_crash() {
        for mode in [
            RenderingMode::Safe,
            RenderingMode::Balanced,
            RenderingMode::Gpu,
        ] {
            assert_eq!(
                web_process_exit_response(
                    &decision_for(mode, true, false),
                    WebProcessExit::TerminatedByApi
                ),
                WebProcessExitResponse::Ignore
            );
        }
    }

    #[test]
    fn other_web_process_exits_are_only_logged() {
        let cases = [
            // Safe has nothing to fall back to.
            (
                decision_for(RenderingMode::Safe, true, false),
                WebProcessExit::Crashed,
            ),
            // SYNARA_RENDERING is the user's to keep.
            (
                decision_for(RenderingMode::Gpu, true, true),
                WebProcessExit::Crashed,
            ),
            (
                decision_for(RenderingMode::Gpu, false, false),
                WebProcessExit::Crashed,
            ),
            // Not a drawing failure.
            (
                decision_for(RenderingMode::Balanced, true, false),
                WebProcessExit::ExceededMemoryLimit,
            ),
            (
                decision_for(RenderingMode::Balanced, true, false),
                WebProcessExit::Unknown,
            ),
        ];
        for (decision, exit) in cases {
            assert_eq!(
                web_process_exit_response(&decision, exit),
                WebProcessExitResponse::Log,
                "{decision:?} {exit:?}"
            );
        }
    }

    #[test]
    fn a_relaunch_drops_only_the_variables_this_process_set() {
        let current = [
            ("WEBKIT_DISABLE_DMABUF_RENDERER", "1"),
            ("__NV_DISABLE_EXPLICIT_SYNC", "1"),
            ("WEBKIT_DISABLE_COMPOSITING_MODE", "0"),
            ("SYNARA_RENDERING_DEFAULT", "balanced"),
            ("GDK_BACKEND", "wayland"),
            ("HOME", "/home/u"),
        ]
        .map(|(name, value)| (OsString::from(name), OsString::from(value)));
        let kept = relaunch_environment(
            current,
            &[
                ("WEBKIT_DISABLE_DMABUF_RENDERER", "1"),
                ("__NV_DISABLE_EXPLICIT_SYNC", "1"),
            ],
        );
        let names: Vec<_> = kept
            .iter()
            .map(|(name, _)| name.to_str().unwrap())
            .collect();
        // WEBKIT_DISABLE_COMPOSITING_MODE came from the user's environment,
        // not from this process, so it stays.
        assert_eq!(
            names,
            [
                "WEBKIT_DISABLE_COMPOSITING_MODE",
                "SYNARA_RENDERING_DEFAULT",
                "GDK_BACKEND",
                "HOME"
            ]
        );
        assert_eq!(kept[0].1, "0");
    }

    #[test]
    fn a_relaunch_with_nothing_set_keeps_the_whole_environment() {
        let current = vec![(OsString::from("A"), OsString::from("1"))];
        assert_eq!(relaunch_environment(current.clone(), &[]), current);
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

    #[test]
    fn a_fallback_is_written_read_back_and_removed() {
        let dir = std::env::temp_dir().join(format!(
            "synara-rendering-fallback-test-{}",
            std::process::id()
        ));
        let path = dir.join("nested").join(FALLBACK_FILE_NAME);
        assert_eq!(read_fallback(&path), None);
        remove_if_present(&path).unwrap();
        write_fallback(&path, BALANCED_CRASHED).unwrap();
        assert_eq!(read_fallback(&path), Some(BALANCED_CRASHED));
        remove_if_present(&path).unwrap();
        assert_eq!(read_fallback(&path), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
