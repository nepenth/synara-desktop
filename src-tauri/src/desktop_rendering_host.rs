//! The app side of `desktop_rendering`: the startup call that sets the
//! rendering variables, the commands behind the "Hardware acceleration"
//! choice in Settings, and the fallback to safe drawing when the web view
//! fails (the startup sentinel and the web process watch).

use crate::desktop_rendering::{
    app_config_dir, decide, fallback_after, nvidia_driver_present, previous_start, read_fallback,
    read_saved_mode, remove_if_present, write_creating_parent, write_fallback, write_saved_mode,
    PreviousStart, RenderingDecision, RenderingFallback, RenderingMode, StartupSentinel,
    FALLBACK_FILE_NAME, PREFERENCE_FILE_NAME, STARTUP_SENTINEL_FILE_NAME,
};
#[cfg(target_os = "linux")]
use crate::desktop_rendering::{web_process_exit_response, WebProcessExit, WebProcessExitResponse};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tauri::Manager;

struct Startup {
    decision: RenderingDecision,
    /// Where the rendering files live; `None` when it cannot be found.
    config_dir: Option<PathBuf>,
}

/// What this process started with. Unset where the setting does not exist.
static STARTUP: OnceLock<Startup> = OnceLock::new();
/// The sentinel this process wrote, until it is removed.
static SENTINEL: Mutex<Option<PathBuf>> = Mutex::new(None);

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopRenderingStatus {
    /// Linux on Wayland only; the other fields mean nothing without it.
    pub available: bool,
    /// The user's choice for the next start; `null` while they have not
    /// chosen and the package's default applies.
    pub saved: Option<RenderingMode>,
    /// The mode this process started with.
    pub active: RenderingMode,
    pub overridden_by_environment: bool,
    /// The recorded failure that keeps starts safe until a mode is chosen.
    pub fallback: Option<RenderingFallback>,
}

/// Set the rendering variables for this process. Call once, before GTK
/// initialises and while the process has a single thread.
///
/// `identifier` is the app identifier of the compiled Tauri config, so a build
/// made with another `--config` identifier reads its own config directory.
pub fn apply_at_startup(identifier: &str) {
    if !cfg!(target_os = "linux") {
        return;
    }
    let is_wayland = crate::desktop_environment::is_wayland();
    let config_dir = if is_wayland {
        app_config_dir(dirs::config_dir(), identifier)
    } else {
        None
    };
    let saved = config_dir
        .as_ref()
        .and_then(|dir| read_saved_mode(&dir.join(PREFERENCE_FILE_NAME)));
    let fallback = config_dir.as_deref().and_then(fallback_for_this_start);
    let decision = decide(
        std::env::consts::OS,
        is_wayland,
        saved,
        |key| std::env::var_os(key).map(|value| value.to_string_lossy().into_owned()),
        nvidia_driver_present(),
        fallback,
    );
    for (name, value) in &decision.set {
        // Changing the environment is only sound while no other thread can
        // read it. `run` calls this before it builds the app: the thread
        // `generate_context!` used has been joined, and GTK, the async
        // runtime and every plugin start later.
        std::env::set_var(name, value);
    }
    if let Some(fallback) = decision.fallback {
        eprintln!(
            "[synara] drawing safely: {} failed earlier ({:?}); choose a mode in Settings to try again",
            fallback.mode.as_str(),
            fallback.reason
        );
    }
    let _ = STARTUP.set(Startup {
        decision,
        config_dir,
    });
}

/// The recorded fallback, after turning a sentinel left by a start that died
/// into one. Removes that sentinel.
fn fallback_for_this_start(dir: &Path) -> Option<RenderingFallback> {
    let fallback_path = dir.join(FALLBACK_FILE_NAME);
    let recorded = read_fallback(&fallback_path);
    let sentinel_path = dir.join(STARTUP_SENTINEL_FILE_NAME);
    let content = std::fs::read_to_string(&sentinel_path).ok();
    let previous = previous_start(content.as_deref(), std::process::id(), same_program_running);
    if matches!(
        previous,
        PreviousStart::Unreadable | PreviousStart::Failed(_)
    ) {
        if let Err(error) = remove_if_present(&sentinel_path) {
            eprintln!("[synara] cannot remove the rendering startup note: {error}");
        }
    }
    let Some(fallback) = fallback_after(previous, recorded) else {
        return recorded;
    };
    eprintln!(
        "[synara] the last start ended before the window had drawn, in {} mode",
        fallback.mode.as_str()
    );
    if let Err(error) = write_fallback(&fallback_path, fallback) {
        // This start still draws safely; the next one tries the mode again.
        eprintln!("[synara] cannot record the rendering fallback: {error}");
    }
    Some(fallback)
}

/// Whether `pid` is another running copy of this program: the process exists
/// and has the same name.
fn same_program_running(pid: u32) -> bool {
    let own = std::fs::read("/proc/self/comm");
    let other = std::fs::read(format!("/proc/{pid}/comm"));
    matches!((own, other), (Ok(own), Ok(other)) if own == other)
}

/// Leaves the startup sentinel when this start may fail. Call from `setup`:
/// a second launch exits in the single-instance plugin before that, so only
/// the copy that shows the window writes one. Nothing has drawn yet then.
pub fn mark_start_in_progress() {
    let Some(startup) = STARTUP.get() else {
        return;
    };
    let (Some(dir), Some(sentinel)) = (
        startup.config_dir.as_ref(),
        StartupSentinel::for_start(&startup.decision, std::process::id()),
    ) else {
        return;
    };
    let path = dir.join(STARTUP_SENTINEL_FILE_NAME);
    match write_creating_parent(&path, &sentinel.file_content()) {
        Ok(()) => {
            if let Ok(mut written) = SENTINEL.lock() {
                *written = Some(path);
            }
        }
        Err(error) => eprintln!("[synara] cannot write the rendering startup note: {error}"),
    }
}

/// Removes the sentinel this process wrote: the interface has drawn, or the
/// app is exiting normally.
pub fn mark_start_finished() {
    let path = SENTINEL.lock().ok().and_then(|mut written| written.take());
    if let Some(path) = path {
        if let Err(error) = remove_if_present(&path) {
            eprintln!("[synara] cannot remove the rendering startup note: {error}");
        }
    }
}

/// The renderer calls this once the app shell has drawn its first frames.
#[tauri::command]
pub fn desktop_rendering_started() {
    mark_start_finished();
}

/// Falls back to safe drawing when the main web view's process crashes in
/// Balanced or Full: records the fallback and starts the app again.
#[cfg(target_os = "linux")]
pub fn watch_web_process<R: tauri::Runtime>(
    window: &tauri::WebviewWindow<R>,
) -> Result<(), String> {
    if !available() {
        return Ok(());
    }
    let app = window.app_handle().clone();
    window
        .with_webview(move |webview| {
            use webkit2gtk::{WebProcessTerminationReason, WebViewExt};

            webview
                .inner()
                .connect_web_process_terminated(move |view, reason| {
                    let exit = match reason {
                        WebProcessTerminationReason::Crashed => WebProcessExit::Crashed,
                        WebProcessTerminationReason::ExceededMemoryLimit => {
                            WebProcessExit::ExceededMemoryLimit
                        }
                        WebProcessTerminationReason::TerminatedByApi => {
                            WebProcessExit::TerminatedByApi
                        }
                        _ => WebProcessExit::Unknown,
                    };
                    if on_web_process_exit(&app, exit) && allow_web_view_reload() {
                        // A web process that ends without a fallback leaves
                        // the window blank; reloading starts a new one.
                        view.reload();
                    }
                });
        })
        .map_err(|error| format!("Unable to watch the web view's process: {error}"))
}

#[cfg(not(target_os = "linux"))]
pub fn watch_web_process<R: tauri::Runtime>(
    _window: &tauri::WebviewWindow<R>,
) -> Result<(), String> {
    Ok(())
}

/// Reloads allowed per window of time after a web process ends, so a page
/// that keeps killing its process cannot loop forever.
#[cfg(any(target_os = "linux", test))]
const WEB_VIEW_RELOADS_PER_WINDOW: usize = 3;
#[cfg(any(target_os = "linux", test))]
const WEB_VIEW_RELOAD_WINDOW: std::time::Duration = std::time::Duration::from_secs(300);

/// Whether one more reload fits the budget at `now`; records it if so.
#[cfg(any(target_os = "linux", test))]
fn take_reload_budget(recent: &mut Vec<std::time::Instant>, now: std::time::Instant) -> bool {
    recent.retain(|at| now.saturating_duration_since(*at) < WEB_VIEW_RELOAD_WINDOW);
    if recent.len() >= WEB_VIEW_RELOADS_PER_WINDOW {
        return false;
    }
    recent.push(now);
    true
}

#[cfg(target_os = "linux")]
fn allow_web_view_reload() -> bool {
    static RECENT: Mutex<Vec<std::time::Instant>> = Mutex::new(Vec::new());
    let allowed = RECENT
        .lock()
        .map(|mut recent| take_reload_budget(&mut recent, std::time::Instant::now()))
        .unwrap_or(false);
    if !allowed {
        eprintln!("[synara] the web view's process keeps ending; not reloading it again");
    }
    allowed
}

/// Handles the end of the web view's process. True when the window is left
/// without a page (no fallback relaunch is under way) and should reload.
#[cfg(target_os = "linux")]
fn on_web_process_exit<R: tauri::Runtime>(app: &tauri::AppHandle<R>, exit: WebProcessExit) -> bool {
    static RELAUNCHING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    let Some(startup) = STARTUP.get() else {
        return !matches!(exit, WebProcessExit::TerminatedByApi);
    };
    let mode = startup.decision.mode.as_str();
    match web_process_exit_response(&startup.decision, exit) {
        WebProcessExitResponse::Ignore => false,
        WebProcessExitResponse::Log => {
            let why = if startup.decision.overridden_by_environment {
                "; SYNARA_RENDERING chose this mode, so it is kept"
            } else {
                ""
            };
            eprintln!("[synara] the web view's process ended ({exit:?}) in {mode} mode{why}");
            true
        }
        WebProcessExitResponse::FallBack(fallback) => {
            if RELAUNCHING.swap(true, std::sync::atomic::Ordering::AcqRel) {
                return false;
            }
            eprintln!(
                "[synara] the web view's process crashed in {mode} mode; starting again with hardware acceleration off"
            );
            // Without the record the new copy would start in the same mode
            // and could crash again, so it is not started; the page reloads
            // in this mode instead.
            let Some(dir) = startup.config_dir.as_ref() else {
                eprintln!("[synara] cannot record the rendering fallback: no config directory");
                return true;
            };
            if let Err(error) = write_fallback(&dir.join(FALLBACK_FILE_NAME), fallback) {
                eprintln!("[synara] cannot record the rendering fallback: {error}");
                return true;
            }
            if let Err(error) = relaunch_after_exit(app, &startup.decision) {
                eprintln!("[synara] cannot start Synara again: {error}");
                return true;
            }
            false
        }
    }
}

/// Exits the app and starts it again once this process is gone. The
/// single-instance plugin would otherwise hand the new copy over to this one.
#[cfg(target_os = "linux")]
fn relaunch_after_exit<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    decision: &RenderingDecision,
) -> Result<(), String> {
    let env = app.env();
    // The AppImage itself when running from one, not its mounted copy.
    let program = tauri::process::current_binary(&env)
        .map_err(|error| format!("cannot find the program: {error}"))?;
    let environment =
        crate::desktop_rendering::relaunch_environment(std::env::vars_os(), &decision.set);
    spawn_relaunch_helper(
        std::process::id(),
        &program,
        env.args_os.iter().skip(1).cloned(),
        environment,
    )
    .map_err(|error| format!("cannot start the relaunch helper: {error}"))?;
    // The exit the shell already handles: sync stops, the process ends, and
    // the helper starts the program.
    app.exit(0);
    Ok(())
}

/// Waits up to 30 s for `pid` to exit (a zombie counts as gone), then runs
/// the program with its arguments. Should the old copy still be alive, the new
/// one hands over to it and exits, so two never run.
#[cfg(target_os = "linux")]
const RELAUNCH_SCRIPT: &str = r#"i=0
while kill -0 "$1" 2>/dev/null && [ "$i" -lt 300 ]; do
  case "$(cat "/proc/$1/stat" 2>/dev/null)" in *") Z "*) break ;; esac
  sleep 0.1; i=$((i+1))
done
shift
exec "$@""#;

#[cfg(target_os = "linux")]
fn relaunch_helper_args(
    pid: u32,
    program: &Path,
    args: impl IntoIterator<Item = std::ffi::OsString>,
) -> Vec<std::ffi::OsString> {
    let mut all = vec![
        "-c".into(),
        RELAUNCH_SCRIPT.into(),
        "synara-relaunch".into(),
        pid.to_string().into(),
        program.as_os_str().to_owned(),
    ];
    all.extend(args);
    all
}

/// Starts the helper with exactly `environment`, in its own process group and
/// with no input, so it outlives this process. Output goes where this
/// process's goes (the journal, for a desktop launch).
#[cfg(target_os = "linux")]
fn spawn_relaunch_helper(
    pid: u32,
    program: &Path,
    args: impl IntoIterator<Item = std::ffi::OsString>,
    environment: Vec<(std::ffi::OsString, std::ffi::OsString)>,
) -> std::io::Result<std::process::Child> {
    use std::os::unix::process::CommandExt;
    std::process::Command::new("/bin/sh")
        .args(relaunch_helper_args(pid, program, args))
        .env_clear()
        .envs(environment)
        .stdin(std::process::Stdio::null())
        .process_group(0)
        .spawn()
}

fn status_with(
    saved: Option<RenderingMode>,
    fallback: Option<RenderingFallback>,
) -> DesktopRenderingStatus {
    match STARTUP.get() {
        Some(startup) if startup.decision.applies => DesktopRenderingStatus {
            available: true,
            saved,
            active: startup.decision.mode,
            overridden_by_environment: startup.decision.overridden_by_environment,
            fallback,
        },
        _ => DesktopRenderingStatus {
            available: false,
            saved: None,
            active: RenderingMode::default(),
            overridden_by_environment: false,
            fallback: None,
        },
    }
}

fn available() -> bool {
    STARTUP
        .get()
        .is_some_and(|startup| startup.decision.applies)
}

fn config_file<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    name: &str,
) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|dir| dir.join(name))
        .map_err(|error| format!("cannot find the settings directory: {error}"))
}

#[tauri::command]
pub fn desktop_rendering_mode(app: tauri::AppHandle) -> DesktopRenderingStatus {
    if !available() {
        return status_with(None, None);
    }
    let read = |name: &str| config_file(&app, name).ok();
    status_with(
        read(PREFERENCE_FILE_NAME).and_then(|path| read_saved_mode(&path)),
        read(FALLBACK_FILE_NAME).and_then(|path| read_fallback(&path)),
    )
}

/// Saves the mode for the next start and forgets a recorded fallback, so the
/// next start tries the mode; this process keeps the one it has.
#[tauri::command]
pub fn desktop_set_rendering_mode(
    app: tauri::AppHandle,
    mode: RenderingMode,
) -> Result<DesktopRenderingStatus, String> {
    if !available() {
        return Err("The rendering setting is only available on Linux with Wayland.".to_owned());
    }
    let path = config_file(&app, PREFERENCE_FILE_NAME)?;
    write_saved_mode(&path, mode)
        .map_err(|error| format!("cannot save the rendering setting: {error}"))?;
    let fallback_path = config_file(&app, FALLBACK_FILE_NAME)?;
    remove_if_present(&fallback_path)
        .map_err(|error| format!("cannot clear the rendering fallback: {error}"))?;
    Ok(status_with(
        read_saved_mode(&path),
        read_fallback(&fallback_path),
    ))
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use std::ffi::OsString;

    #[test]
    fn the_reload_budget_allows_a_few_reloads_per_window_then_refills() {
        let start = std::time::Instant::now();
        let mut recent = Vec::new();
        for _ in 0..WEB_VIEW_RELOADS_PER_WINDOW {
            assert!(take_reload_budget(&mut recent, start));
        }
        assert!(!take_reload_budget(&mut recent, start));
        let later = start + WEB_VIEW_RELOAD_WINDOW;
        assert!(take_reload_budget(&mut recent, later));
    }

    /// A fresh directory for one test, removed when dropped.
    struct TestDir(PathBuf);

    impl TestDir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "synara-rendering-host-{name}-{}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            TestDir(dir)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn the_relaunch_helper_is_given_the_pid_the_program_and_its_arguments() {
        let args = relaunch_helper_args(
            4242,
            Path::new("/usr/lib/synara/synara"),
            [OsString::from("synara://room/x"), OsString::from("--flag")],
        );
        assert_eq!(args[0], "-c");
        assert!(args[1].to_str().unwrap().contains("kill -0 \"$1\""));
        assert!(args[1].to_str().unwrap().ends_with("shift\nexec \"$@\""));
        assert_eq!(args[2], "synara-relaunch");
        assert_eq!(args[3], "4242");
        assert_eq!(args[4], "/usr/lib/synara/synara");
        assert_eq!(args[5], "synara://room/x");
        assert_eq!(args[6], "--flag");
        assert_eq!(args.len(), 7);
    }

    #[test]
    fn the_relaunch_helper_waits_for_the_old_process_then_runs_the_program() {
        let root = TestDir::new("wait");
        let ran = root.path().join("ran");
        let program = root.path().join("fake-synara");
        std::fs::write(
            &program,
            format!(
                "#!/bin/sh\nprintf '%s|%s|%s' \"$1\" \"${{KEPT:-}}\" \"${{DROPPED:-unset}}\" > '{}'\n",
                ran.display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(
            &program,
            std::os::unix::fs::PermissionsExt::from_mode(0o755),
        )
        .unwrap();

        // Stands in for the exiting app.
        let mut old_instance = std::process::Command::new("sleep")
            .arg("0.4")
            .spawn()
            .unwrap();
        let started = std::time::Instant::now();
        let environment = crate::desktop_rendering::relaunch_environment(
            [
                (OsString::from("KEPT"), OsString::from("yes")),
                (OsString::from("DROPPED"), OsString::from("1")),
                (
                    OsString::from("PATH"),
                    std::env::var_os("PATH").unwrap_or_default(),
                ),
            ],
            &[("DROPPED", "1")],
        );
        let mut helper = spawn_relaunch_helper(
            old_instance.id(),
            &program,
            [OsString::from("arg one")],
            environment,
        )
        .unwrap();
        // Reaping the old process mid-wait is what its launcher does.
        old_instance.wait().unwrap();

        assert!(helper.wait().unwrap().success());
        assert!(
            started.elapsed() >= std::time::Duration::from_millis(350),
            "the helper started the program before the old process had exited"
        );
        assert_eq!(std::fs::read_to_string(&ran).unwrap(), "arg one|yes|unset");
    }

    #[test]
    fn the_relaunch_helper_does_not_wait_for_a_zombie() {
        let root = TestDir::new("zombie");
        let ran = root.path().join("ran");
        let program = root.path().join("fake-synara");
        std::fs::write(&program, format!("#!/bin/sh\ntouch '{}'\n", ran.display())).unwrap();
        std::fs::set_permissions(
            &program,
            std::os::unix::fs::PermissionsExt::from_mode(0o755),
        )
        .unwrap();
        // Exits at once and is not reaped until the end: a zombie.
        let mut zombie = std::process::Command::new("true").spawn().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(200));
        let started = std::time::Instant::now();
        let mut helper = spawn_relaunch_helper(
            zombie.id(),
            &program,
            [],
            vec![(
                OsString::from("PATH"),
                std::env::var_os("PATH").unwrap_or_default(),
            )],
        )
        .unwrap();
        assert!(helper.wait().unwrap().success());
        assert!(started.elapsed() < std::time::Duration::from_secs(5));
        assert!(ran.exists());
        zombie.wait().unwrap();
    }

    #[test]
    fn this_program_is_running_and_a_missing_process_is_not() {
        assert!(same_program_running(std::process::id()));
        assert!(!same_program_running(u32::MAX));
    }
}
