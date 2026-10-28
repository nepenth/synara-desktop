use serde::Serialize;
#[cfg(debug_assertions)]
use std::sync::atomic::AtomicU64;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
#[cfg(target_os = "macos")]
use tauri::image::Image;
use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Runtime};

use crate::build_info;
use crate::desktop::{
    navigate_main_window, show_main_window, MAIN_WINDOW_LABEL, ROUTE_HOME, ROUTE_LATER,
    ROUTE_NOTIFICATIONS, ROUTE_SETTINGS,
};
use crate::desktop_unread_badge;

const MENU_SHOW: &str = "desktop.show";
const MENU_LATER: &str = "desktop.later";
const MENU_NOTIFICATIONS: &str = "desktop.notifications";
const MENU_UNREAD_SUMMARY: &str = "desktop.unread-summary";
const MENU_DESKTOP_INTEGRATION: &str = "desktop.integration";
const MENU_DND_TOGGLE: &str = "desktop.dnd";
const MENU_BUILD_INFO: &str = "desktop.build-info";
const MENU_QUIT: &str = "desktop.quit";

pub const DESKTOP_TRAY_DND_TOGGLE_EVENT: &str = "synara-tray-dnd-toggle";

pub(crate) const TRAY_ICON_ID: &str = "synara-tray";
const TRAY_STATE_APPLY_MIN_INTERVAL_MS: u64 = 500;
const MAX_TRAY_COUNT: i64 = 9_999;

#[cfg(debug_assertions)]
static TRAY_MENU_REBUILD_COUNT: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopTrayState {
    pub unread_count: i64,
    pub highlight_count: i64,
    pub later_count: i64,
    pub notification_inbox_count: i64,
    pub do_not_disturb: bool,
}

struct TrayMenuItems<R: Runtime> {
    later: MenuItem<R>,
    notifications: MenuItem<R>,
    #[cfg(target_os = "linux")]
    unread_summary: MenuItem<R>,
    dnd: MenuItem<R>,
}

struct TrayStateCoalescer {
    pending: Mutex<Option<DesktopTrayState>>,
    last_applied_at: Mutex<Option<Instant>>,
    flush_scheduled: AtomicBool,
}

impl TrayStateCoalescer {
    fn new() -> Self {
        Self {
            pending: Mutex::new(None),
            last_applied_at: Mutex::new(None),
            flush_scheduled: AtomicBool::new(false),
        }
    }
}

#[cfg(debug_assertions)]
#[allow(dead_code)]
pub fn debug_tray_menu_rebuild_count() -> u64 {
    TRAY_MENU_REBUILD_COUNT.load(Ordering::Relaxed)
}

fn tray_state_apply_min_interval() -> Duration {
    Duration::from_millis(TRAY_STATE_APPLY_MIN_INTERVAL_MS)
}

fn should_apply_tray_state_now(last_applied_at: Option<Instant>, now: Instant) -> bool {
    last_applied_at
        .map(|applied_at| now.duration_since(applied_at) >= tray_state_apply_min_interval())
        .unwrap_or(true)
}

fn clamp_count(value: i64) -> i64 {
    value.clamp(0, MAX_TRAY_COUNT)
}

fn normalize_tray_state(state: DesktopTrayState) -> DesktopTrayState {
    DesktopTrayState {
        unread_count: clamp_count(state.unread_count),
        highlight_count: clamp_count(state.highlight_count),
        later_count: clamp_count(state.later_count),
        notification_inbox_count: clamp_count(state.notification_inbox_count),
        do_not_disturb: state.do_not_disturb,
    }
}

/// Same contract as JS `summarizeNotifications` appBadgeCount: later +
/// highlights + ordinary unreads. Invites/agent stay on the inbox, not the icon.
fn tray_badge_count(state: &DesktopTrayState) -> i64 {
    clamp_count(
        clamp_count(state.unread_count)
            .saturating_add(clamp_count(state.highlight_count))
            .saturating_add(clamp_count(state.later_count)),
    )
}

fn tray_title_for_badge(_count: i64) -> Option<String> {
    // The menu and tooltip carry the count. A title beside a panel icon is
    // too small to read and duplicates the dot on the icon itself.
    None
}

fn apply_linux_tray_badge_title<R: Runtime>(
    tray: &tauri::tray::TrayIcon<R>,
    count: i64,
) -> tauri::Result<()> {
    let title = tray_title_for_badge(count);
    #[cfg(target_os = "linux")]
    tray.set_title(title)?;
    #[cfg(not(target_os = "linux"))]
    let _ = (tray, title);
    Ok(())
}

#[cfg(target_os = "linux")]
fn apply_linux_tray_badge_icon<R: Runtime>(
    app: &AppHandle<R>,
    tray: &tauri::tray::TrayIcon<R>,
    count: i64,
) -> tauri::Result<()> {
    let Some(base) = app.default_window_icon() else {
        return Ok(());
    };
    if count <= 0 {
        tray.set_icon(Some(base.clone()))?;
        return Ok(());
    }
    let Some(rgba) =
        desktop_unread_badge::overlay_unread_dot(base.rgba(), base.width(), base.height(), count)
    else {
        tray.set_icon(Some(base.clone()))?;
        return Ok(());
    };
    let badged = tauri::image::Image::new_owned(rgba, base.width(), base.height());
    tray.set_icon(Some(badged))?;
    Ok(())
}

fn apply_linux_unread_surfaces<R: Runtime>(
    app: &AppHandle<R>,
    tray: &tauri::tray::TrayIcon<R>,
    count: i64,
) -> tauri::Result<()> {
    apply_linux_tray_badge_title(tray, count)?;
    #[cfg(target_os = "linux")]
    apply_linux_tray_badge_icon(app, tray, count)?;
    #[cfg(not(target_os = "linux"))]
    let _ = app;
    desktop_unread_badge::emit_unity_launcher_badge(count);
    Ok(())
}

fn tray_route_labels(state: &DesktopTrayState) -> [String; 5] {
    let unread = clamp_count(state.unread_count);
    let highlights = clamp_count(state.highlight_count);
    let later = clamp_count(state.later_count);
    let notifications = clamp_count(state.notification_inbox_count);
    let badge = tray_badge_count(state);
    let do_not_disturb = state.do_not_disturb;
    let summary = format!(
        "Badge: {badge} | Unread: {unread} | Highlights: {highlights} | Later: {later} | Notifications: {notifications}"
    );
    let later_label = format!("Later ({later})");
    let notifications_label = format!("Notifications ({notifications})");
    let dnd_label = if do_not_disturb {
        "Do Not Disturb: On (Approvals Still Alert)"
    } else {
        "Do Not Disturb: Off"
    };
    let integration_label = "Desktop Integration";
    [
        summary,
        later_label,
        notifications_label,
        dnd_label.to_owned(),
        integration_label.to_owned(),
    ]
}

fn apply_tray_menu_labels<R: Runtime>(
    items: &TrayMenuItems<R>,
    state: &DesktopTrayState,
) -> tauri::Result<()> {
    let route_labels = tray_route_labels(state);
    items.later.set_text(route_labels[1].as_str())?;
    items.notifications.set_text(route_labels[2].as_str())?;
    items.dnd.set_text(route_labels[3].as_str())?;
    #[cfg(target_os = "linux")]
    items.unread_summary.set_text(route_labels[0].as_str())?;
    Ok(())
}

fn apply_tray_state_in_place<R: Runtime>(
    app: &AppHandle<R>,
    items: &TrayMenuItems<R>,
    state: &DesktopTrayState,
) -> Result<(), String> {
    apply_tray_menu_labels(items, state).map_err(|error| error.to_string())?;
    if let Some(tray) = app.tray_by_id(TRAY_ICON_ID) {
        tray.set_tooltip(Some(tray_tooltip(state)))
            .map_err(|error| error.to_string())?;
        apply_linux_unread_surfaces(app, &tray, tray_badge_count(state))
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn rebuild_tray_menu<R: Runtime>(
    app: &AppHandle<R>,
    state: &DesktopTrayState,
) -> Result<(), String> {
    let Some(tray) = app.tray_by_id(TRAY_ICON_ID) else {
        return Ok(());
    };

    #[cfg(debug_assertions)]
    TRAY_MENU_REBUILD_COUNT.fetch_add(1, Ordering::Relaxed);

    let built_menu = build_tray_menu(app, state).map_err(|error| error.to_string())?;
    tray.set_menu(Some(built_menu.0))
        .map_err(|error| error.to_string())?;
    tray.set_tooltip(Some(tray_tooltip(state)))
        .map_err(|error| error.to_string())?;
    apply_linux_unread_surfaces(app, &tray, tray_badge_count(state))
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn apply_pending_tray_state<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let coalescer = app.state::<TrayStateCoalescer>();
    let state = {
        let mut pending = coalescer
            .pending
            .lock()
            .map_err(|error| error.to_string())?;
        pending.take()
    };
    let Some(state) = state else {
        return Ok(());
    };

    let normalized = normalize_tray_state(state);
    if let Some(items) = app.try_state::<TrayMenuItems<R>>() {
        apply_tray_state_in_place(app, &items, &normalized)?;
    } else {
        rebuild_tray_menu(app, &normalized)?;
    }

    let mut last_applied_at = coalescer
        .last_applied_at
        .lock()
        .map_err(|error| error.to_string())?;
    *last_applied_at = Some(Instant::now());
    Ok(())
}

fn schedule_tray_state_flush<R: Runtime>(app: AppHandle<R>) {
    let coalescer = app.state::<TrayStateCoalescer>();
    if coalescer.flush_scheduled.swap(true, Ordering::AcqRel) {
        return;
    }

    let delay = {
        let last_applied_at = coalescer
            .last_applied_at
            .lock()
            .ok()
            .and_then(|guard| *guard);
        let elapsed = last_applied_at
            .map(|applied_at| Instant::now().duration_since(applied_at))
            .unwrap_or(tray_state_apply_min_interval());
        tray_state_apply_min_interval()
            .checked_sub(elapsed)
            .unwrap_or(Duration::ZERO)
    };

    tauri::async_runtime::spawn(async move {
        if !delay.is_zero() {
            tokio::time::sleep(delay).await;
        }

        let coalescer = app.state::<TrayStateCoalescer>();
        coalescer.flush_scheduled.store(false, Ordering::Release);
        if let Err(error) = apply_pending_tray_state(&app) {
            log_tray_error(
                &app,
                &format!("failed to apply coalesced tray state: {error}"),
            );
        }
    });
}

fn queue_tray_state_update<R: Runtime>(
    app: AppHandle<R>,
    state: DesktopTrayState,
) -> Result<(), String> {
    let coalescer = app.state::<TrayStateCoalescer>();
    {
        let mut pending = coalescer
            .pending
            .lock()
            .map_err(|error| error.to_string())?;
        *pending = Some(state);
    }

    let now = Instant::now();
    let apply_now = {
        let last_applied_at = coalescer
            .last_applied_at
            .lock()
            .map_err(|error| error.to_string())?;
        should_apply_tray_state_now(*last_applied_at, now)
    };

    if apply_now {
        apply_pending_tray_state(&app)
    } else {
        schedule_tray_state_flush(app);
        Ok(())
    }
}

fn build_tray_menu<R: Runtime>(
    app: &AppHandle<R>,
    state: &DesktopTrayState,
) -> tauri::Result<(Menu<R>, TrayMenuItems<R>)> {
    let route_labels = tray_route_labels(state);

    // No accelerators: the global shortcuts are user-set (or off), so a
    // fixed hint here would be wrong.
    let show = MenuItem::with_id(app, MENU_SHOW, "Show Synara", true, None::<&str>)?;
    let later = MenuItem::with_id(
        app,
        MENU_LATER,
        route_labels[1].as_str(),
        true,
        None::<&str>,
    )?;
    let notifications = MenuItem::with_id(
        app,
        MENU_NOTIFICATIONS,
        route_labels[2].as_str(),
        true,
        None::<&str>,
    )?;
    // Every platform's tray can turn Synara's notifications off and on; the
    // menu bar icon is the quickest place to reach it on macOS too.
    let dnd = MenuItem::with_id(
        app,
        MENU_DND_TOGGLE,
        route_labels[3].as_str(),
        true,
        None::<&str>,
    )?;
    let separator = PredefinedMenuItem::separator(app)?;
    let build_item = MenuItem::with_id(
        app,
        MENU_BUILD_INFO,
        build_info::menu_label(),
        false,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "Quit Synara", true, Some("CmdOrCtrl+Q"))?;

    #[cfg(not(target_os = "linux"))]
    let menu = Menu::with_items(
        app,
        &[
            &show,
            &later,
            &notifications,
            &dnd,
            &separator,
            &build_item,
            &quit,
        ],
    )?;
    #[cfg(not(target_os = "linux"))]
    let items = TrayMenuItems {
        later,
        notifications,
        dnd,
    };

    #[cfg(target_os = "linux")]
    let unread_summary = MenuItem::with_id(
        app,
        MENU_UNREAD_SUMMARY,
        route_labels[0].as_str(),
        false,
        None::<&str>,
    )?;
    #[cfg(target_os = "linux")]
    let desktop_integration = MenuItem::with_id(
        app,
        MENU_DESKTOP_INTEGRATION,
        route_labels[4].as_str(),
        true,
        None::<&str>,
    )?;
    #[cfg(target_os = "linux")]
    let menu = Menu::with_items(
        app,
        &[
            &show,
            &unread_summary,
            &later,
            &notifications,
            &desktop_integration,
            &dnd,
            &separator,
            &build_item,
            &quit,
        ],
    )?;
    #[cfg(target_os = "linux")]
    let items = TrayMenuItems {
        later,
        notifications,
        unread_summary,
        dnd,
    };

    Ok((menu, items))
}

fn tray_tooltip(state: &DesktopTrayState) -> String {
    let unread = clamp_count(state.unread_count);
    let highlights = clamp_count(state.highlight_count);
    let later = clamp_count(state.later_count);
    let badge = tray_badge_count(state);
    format!("Synara — {badge} ({unread} unread, {highlights} highlights, {later} later)")
}

pub fn tray_dnd_toggle_dispatch_script() -> String {
    format!("window.dispatchEvent(new CustomEvent('{DESKTOP_TRAY_DND_TOGGLE_EVENT}'));")
}

fn emit_tray_dnd_toggle<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        window.eval(tray_dnd_toggle_dispatch_script())?;
    }
    Ok(())
}

pub fn set_badge_count<R: Runtime>(app: &AppHandle<R>, count: Option<i64>) -> tauri::Result<()> {
    let normalized_count = count.map(clamp_count).filter(|value| *value > 0);
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        // Windows has no taskbar count; it overlays a small icon instead.
        #[cfg(target_os = "windows")]
        window.set_overlay_icon(normalized_count.map(|_| {
            tauri::image::Image::new_owned(desktop_unread_badge::unread_dot_icon(16), 16, 16)
        }))?;
        #[cfg(not(target_os = "windows"))]
        window.set_badge_count(normalized_count)?;

        #[cfg(target_os = "macos")]
        {
            window.set_badge_label(normalized_count.map(|value| value.to_string()))?;
        }
    }

    // Unity `set_badge_count` is a no-op on GNOME (libunity only acts when
    // Unity itself is running). Linux paints a dot on the panel icon and
    // emits the numeric LauncherEntry signal for supporting docks.
    if let Some(tray) = app.tray_by_id(TRAY_ICON_ID) {
        apply_linux_unread_surfaces(app, &tray, normalized_count.unwrap_or(0))?;
    } else {
        desktop_unread_badge::emit_unity_launcher_badge(normalized_count.unwrap_or(0));
    }
    Ok(())
}

pub fn create_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    app.manage(TrayStateCoalescer::new());

    let initial_state = DesktopTrayState {
        unread_count: 0,
        highlight_count: 0,
        later_count: 0,
        notification_inbox_count: 0,
        do_not_disturb: false,
    };
    let (menu, tray_items) = build_tray_menu(app, &initial_state)?;
    app.manage(tray_items);

    let mut builder = TrayIconBuilder::with_id(TRAY_ICON_ID)
        .tooltip(tray_tooltip(&initial_state))
        .menu(&menu)
        // Linux: left-click restores the running window (right-click keeps the
        // menu). Other platforms keep the historical click-to-menu extra.
        .show_menu_on_left_click(cfg!(not(target_os = "linux")))
        .on_menu_event(handle_menu_event);

    #[cfg(target_os = "linux")]
    {
        builder = builder.on_tray_icon_event(|tray, event| {
            use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                if let Err(error) = show_main_window(tray.app_handle()) {
                    log_tray_error(
                        tray.app_handle(),
                        &format!("failed to show Synara from the tray: {error}"),
                    );
                }
            }
        });
    }

    #[cfg(target_os = "macos")]
    {
        let tray_icon = Image::from_bytes(include_bytes!("../icons/tray-template.png"))?;
        builder = builder.icon(tray_icon).icon_as_template(true);
    }

    #[cfg(not(target_os = "macos"))]
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone()).icon_as_template(false);
    }

    builder.build(app)?;
    Ok(())
}

pub fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    let result = match event.id().as_ref() {
        MENU_SHOW => show_main_window(app),
        MENU_LATER => navigate_main_window(app, ROUTE_LATER),
        MENU_NOTIFICATIONS => navigate_main_window(app, ROUTE_NOTIFICATIONS),
        MENU_UNREAD_SUMMARY => navigate_main_window(app, ROUTE_HOME),
        MENU_DESKTOP_INTEGRATION => navigate_main_window(app, ROUTE_SETTINGS),
        MENU_DND_TOGGLE => emit_tray_dnd_toggle(app),
        MENU_BUILD_INFO => Ok(()),
        MENU_QUIT => {
            app.exit(0);
            Ok(())
        }
        _ => Ok(()),
    };

    if let Err(error) = result {
        log_tray_error(
            app,
            &format!("failed to handle desktop menu event: {error}"),
        );
    }
}

/// Tray failures go to the app log as well as stderr.
fn log_tray_error<R: Runtime>(app: &AppHandle<R>, message: &str) {
    eprintln!("[synara] {message}");
    crate::desktop_logging::append_app_log(app, "tray", message);
}

pub(crate) fn tray_is_available<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.tray_by_id(TRAY_ICON_ID).is_some() && tray_host_state() != TrayHostState::Absent
}

/// Whether the desktop shows tray icons. Linux panels show one only through
/// a StatusNotifier host (KDE, or GNOME with the AppIndicator extension); a
/// stock GNOME session has none, so the icon and its Quit item are invisible.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TrayHostState {
    /// Not probed yet, or not Linux: treated as present.
    Unknown,
    Present,
    Absent,
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
static TRAY_HOST_STATE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

pub(crate) fn tray_host_state() -> TrayHostState {
    match TRAY_HOST_STATE.load(Ordering::Acquire) {
        1 => TrayHostState::Present,
        2 => TrayHostState::Absent,
        _ => TrayHostState::Unknown,
    }
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn set_tray_host_state(present: bool) {
    TRAY_HOST_STATE.store(if present { 1 } else { 2 }, Ordering::Release);
}

/// What closing the main window does: hide to the tray, or quit when there
/// is no tray to come back from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MainWindowCloseAction {
    Hide,
    Quit,
}

pub(crate) fn main_window_close_action(host: TrayHostState) -> MainWindowCloseAction {
    if host == TrayHostState::Absent {
        MainWindowCloseAction::Quit
    } else {
        MainWindowCloseAction::Hide
    }
}

/// How often the Linux tray host is re-checked; a host can start after
/// Synara (session autostart) or go away (extension turned off).
#[cfg(target_os = "linux")]
const TRAY_HOST_PROBE_INTERVAL: Duration = Duration::from_secs(30);

/// Probe the StatusNotifier host now and then every
/// [`TRAY_HOST_PROBE_INTERVAL`], off the main thread.
#[cfg(target_os = "linux")]
pub(crate) fn start_tray_host_watch() {
    let spawned = std::thread::Builder::new()
        .name("synara-tray-host-probe".to_owned())
        .spawn(|| loop {
            set_tray_host_state(status_notifier_host_registered());
            std::thread::sleep(TRAY_HOST_PROBE_INTERVAL);
        });
    if spawned.is_err() {
        eprintln!("[synara] Unable to start the tray host probe thread.");
    }
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn start_tray_host_watch() {}

/// `org.kde.StatusNotifierWatcher.IsStatusNotifierHostRegistered`. No session
/// bus or no watcher means no host.
#[cfg(target_os = "linux")]
fn status_notifier_host_registered() -> bool {
    let Ok(connection) = zbus::blocking::Connection::session() else {
        return false;
    };
    let Ok(proxy) = zbus::blocking::Proxy::new(
        &connection,
        "org.kde.StatusNotifierWatcher",
        "/StatusNotifierWatcher",
        "org.kde.StatusNotifierWatcher",
    ) else {
        return false;
    };
    proxy
        .get_property::<bool>("IsStatusNotifierHostRegistered")
        .unwrap_or(false)
}

/// The renderer's last Do Not Disturb state, kept so a reset after sign-out
/// clears the counts without flipping the toggle's label.
static TRAY_DO_NOT_DISTURB: AtomicBool = AtomicBool::new(false);

pub(crate) fn update_tray_state<R: Runtime>(
    app: AppHandle<R>,
    state: DesktopTrayState,
) -> Result<bool, String> {
    TRAY_DO_NOT_DISTURB.store(state.do_not_disturb, Ordering::Relaxed);
    if tray_is_available(&app) {
        queue_tray_state_update(app, state)?;
        return Ok(true);
    }

    Ok(false)
}

/// Signed out: zero every tray count. The signed-out shell sends no tray
/// state, so without this the old account's counts would stay.
pub(crate) fn reset_tray_counts<R: Runtime>(app: &AppHandle<R>) -> Result<bool, String> {
    update_tray_state(app.clone(), signed_out_tray_state())
}

fn signed_out_tray_state() -> DesktopTrayState {
    DesktopTrayState {
        unread_count: 0,
        highlight_count: 0,
        later_count: 0,
        notification_inbox_count: 0,
        do_not_disturb: TRAY_DO_NOT_DISTURB.load(Ordering::Relaxed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tray_state_apply_interval_allows_first_update_and_blocks_rapid_rebuilds() {
        let interval = tray_state_apply_min_interval();
        let started_at = Instant::now();
        assert!(should_apply_tray_state_now(None, started_at));
        assert!(!should_apply_tray_state_now(
            Some(started_at),
            started_at + Duration::from_millis(100)
        ));
        assert!(should_apply_tray_state_now(
            Some(started_at),
            started_at + interval
        ));
    }

    #[test]
    fn normalize_tray_state_clamps_all_count_fields() {
        let normalized = normalize_tray_state(DesktopTrayState {
            unread_count: -3,
            highlight_count: 12_345,
            later_count: 4,
            notification_inbox_count: -1,
            do_not_disturb: true,
        });

        assert_eq!(normalized.unread_count, 0);
        assert_eq!(normalized.highlight_count, 9_999);
        assert_eq!(normalized.later_count, 4);
        assert_eq!(normalized.notification_inbox_count, 0);
        assert!(normalized.do_not_disturb);
    }

    #[test]
    fn badge_count_uses_same_clamp_as_tray_state() {
        assert_eq!(clamp_count(50_000), 9_999);
        assert_eq!(clamp_count(-3), 0);
    }

    #[test]
    fn tray_state_fields_are_clamped() {
        assert_eq!(clamp_count(-1), 0);
        assert_eq!(clamp_count(15_000), 9_999);
        assert_eq!(clamp_count(23), 23);

        let labels = tray_route_labels(&DesktopTrayState {
            unread_count: -5,
            highlight_count: 12_000,
            later_count: 3,
            notification_inbox_count: -9,
            do_not_disturb: true,
        });
        assert!(labels[0].contains("Unread: 0"));
        assert!(labels[0].contains("Highlights: 9999"));
        assert!(labels[0].contains("Later: 3"));
        assert!(labels[0].contains("Notifications: 0"));
    }

    #[test]
    fn tray_route_labels_reflect_do_not_disturb_state() {
        let on = tray_route_labels(&DesktopTrayState {
            unread_count: 0,
            highlight_count: 0,
            later_count: 0,
            notification_inbox_count: 0,
            do_not_disturb: true,
        });
        let off = tray_route_labels(&DesktopTrayState {
            unread_count: 0,
            highlight_count: 0,
            later_count: 0,
            notification_inbox_count: 0,
            do_not_disturb: false,
        });

        assert_eq!(on[3], "Do Not Disturb: On (Approvals Still Alert)");
        assert_eq!(off[3], "Do Not Disturb: Off");
    }

    #[test]
    fn tray_dnd_toggle_dispatch_script_emits_custom_event() {
        assert_eq!(
            tray_dnd_toggle_dispatch_script(),
            "window.dispatchEvent(new CustomEvent('synara-tray-dnd-toggle'));"
        );
    }

    #[test]
    fn tray_badge_count_matches_js_app_badge_contract() {
        let state = DesktopTrayState {
            unread_count: 3,
            highlight_count: 2,
            later_count: 5,
            notification_inbox_count: 99,
            do_not_disturb: false,
        };
        assert_eq!(tray_badge_count(&state), 10);
    }

    #[test]
    fn tray_title_clears_when_count_is_zero() {
        assert_eq!(tray_title_for_badge(0), None);
        assert_eq!(tray_title_for_badge(-4), None);
        assert_eq!(tray_title_for_badge(7), None);
        assert_eq!(tray_title_for_badge(50_000), None);
    }

    #[test]
    fn linux_unread_summary_includes_visible_badge_count() {
        let state = DesktopTrayState {
            unread_count: 3,
            highlight_count: 2,
            later_count: 5,
            notification_inbox_count: 1,
            do_not_disturb: false,
        };
        let labels = tray_route_labels(&state);
        assert!(labels[0].contains("Badge: 10"));
        assert!(labels[0].contains("Unread: 3"));
        assert!(labels[0].contains("Highlights: 2"));
        assert!(labels[0].contains("Later: 5"));
        assert!(labels[0].contains("Notifications: 1"));
        assert_eq!(
            tray_tooltip(&state),
            "Synara — 10 (3 unread, 2 highlights, 5 later)"
        );
        assert_eq!(tray_title_for_badge(tray_badge_count(&state)), None);
    }

    #[test]
    fn close_quits_only_when_no_tray_host_can_show_the_icon() {
        assert_eq!(
            main_window_close_action(TrayHostState::Absent),
            MainWindowCloseAction::Quit
        );
        assert_eq!(
            main_window_close_action(TrayHostState::Present),
            MainWindowCloseAction::Hide
        );
        // Before the first probe (and on macOS/Windows) the tray is assumed.
        assert_eq!(
            main_window_close_action(TrayHostState::Unknown),
            MainWindowCloseAction::Hide
        );
    }

    #[test]
    fn linux_quit_stays_reachable_without_a_tray_host() {
        // Close quits without a host, Ctrl+Q quits from the in-app strip, and
        // the integration panel reports the tray as unavailable.
        let desktop = include_str!("desktop.rs");
        let close_fn = desktop
            .split("pub fn close_main_window")
            .nth(1)
            .and_then(|rest| rest.split("pub fn ").next())
            .expect("close_main_window");
        assert!(close_fn.contains("main_window_close_action(desktop_tray::tray_host_state())"));
        assert!(close_fn.contains("app.exit(0)"));
        assert!(desktop.contains("pub fn desktop_quit"));
        let lib = include_str!("lib.rs");
        assert!(lib.contains("desktop::close_main_window(window.app_handle())"));
        assert!(lib.contains("desktop_tray::start_tray_host_watch()"));
        let source = include_str!("desktop_tray.rs");
        assert!(source.contains("\"IsStatusNotifierHostRegistered\""));
        assert!(source.contains("tray_host_state() != TrayHostState::Absent"));
        let titlebar =
            include_str!("../../synara/src/app/features/desktop-titlebar/DesktopTitleBar.tsx");
        assert!(titlebar.contains("'desktop_quit'"));
    }

    #[test]
    fn sign_out_zeroes_tray_counts_but_keeps_do_not_disturb() {
        TRAY_DO_NOT_DISTURB.store(true, Ordering::Relaxed);
        let state = signed_out_tray_state();
        assert_eq!(state.unread_count, 0);
        assert_eq!(state.highlight_count, 0);
        assert_eq!(state.later_count, 0);
        assert_eq!(state.notification_inbox_count, 0);
        assert!(state.do_not_disturb);
        TRAY_DO_NOT_DISTURB.store(false, Ordering::Relaxed);
        assert!(!signed_out_tray_state().do_not_disturb);
        // matrix_logout resets the tray, badge and banners itself, so forced
        // and expired sign-outs are covered without the renderer.
        let logout = include_str!("matrix/auth/product_commands.rs")
            .split("pub async fn matrix_logout")
            .nth(1)
            .and_then(|rest| rest.split("pub(super) struct LogoutPlan").next())
            .expect("logout body");
        assert!(logout.contains("desktop_notifications::clear_signed_out_surfaces(&app)"));
    }

    #[test]
    fn every_tray_menu_offers_do_not_disturb() {
        let build = include_str!("desktop_tray.rs")
            .split("fn build_tray_menu")
            .nth(1)
            .and_then(|rest| rest.split("fn tray_tooltip").next())
            .expect("build_tray_menu");
        let (mac, linux) = build
            .split_once("#[cfg(target_os = \"linux\")]\n    let unread_summary")
            .expect("platform menus");
        // The non-Linux (macOS) menu and items include the toggle.
        assert!(mac.contains("&notifications,\n            &dnd,"));
        assert!(mac.contains("notifications,\n        dnd,\n    };"));
        assert!(linux.contains("&dnd,"));
        assert!(!build.contains("#[cfg(target_os = \"linux\")]\n    let dnd"));
    }

    #[test]
    fn tray_menu_items_carry_no_fixed_global_shortcut_hints() {
        let source = include_str!("desktop_tray.rs");
        let build = source
            .split("fn build_tray_menu")
            .nth(1)
            .and_then(|rest| rest.split("fn tray_tooltip").next())
            .expect("build_tray_menu");
        assert!(!build.contains("Shift+"));
    }

    #[test]
    fn linux_tray_left_click_restores_instead_of_opening_the_menu() {
        // Only the production half: a needle in this test would match itself.
        let source = include_str!("desktop_tray.rs")
            .split("#[cfg(test)]\nmod tests")
            .next()
            .expect("production source");
        assert!(source.contains("show_menu_on_left_click(cfg!(not(target_os = \"linux\")))"));
        assert!(source.contains("MouseButton::Left"));
        assert!(source.contains("desktop_unread_badge::overlay_unread_dot("));
        assert!(source.contains("desktop_unread_badge::emit_unity_launcher_badge("));
    }
}
