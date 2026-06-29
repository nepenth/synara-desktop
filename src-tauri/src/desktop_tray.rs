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
    #[cfg(target_os = "linux")]
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
    match value {
        value if value < 0 => 0,
        value if value > MAX_TRAY_COUNT => MAX_TRAY_COUNT,
        value => value,
    }
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

fn tray_route_labels(state: &DesktopTrayState) -> [String; 5] {
    let unread = clamp_count(state.unread_count);
    let highlights = clamp_count(state.highlight_count);
    let later = clamp_count(state.later_count);
    let notifications = clamp_count(state.notification_inbox_count);
    let do_not_disturb = state.do_not_disturb;
    let summary = format!(
        "Unread: {unread} | Highlights: {highlights} | Later: {later} | Notifications: {notifications}"
    );
    let later_label = format!("Later ({later})");
    let notifications_label = format!("Notifications ({notifications})");
    let dnd_label = if do_not_disturb {
        "Do Not Disturb: On"
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
    #[cfg(target_os = "linux")]
    {
        items.unread_summary.set_text(route_labels[0].as_str())?;
        items.dnd.set_text(route_labels[3].as_str())?;
    }
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
            let _ = tauri::async_runtime::spawn_blocking(move || std::thread::sleep(delay)).await;
        }

        let coalescer = app.state::<TrayStateCoalescer>();
        coalescer.flush_scheduled.store(false, Ordering::Release);
        if let Err(error) = apply_pending_tray_state(&app) {
            eprintln!("failed to apply coalesced tray state: {error}");
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

    let show = MenuItem::with_id(
        app,
        MENU_SHOW,
        "Show Synara",
        true,
        Some("CmdOrCtrl+Shift+C"),
    )?;
    let later = MenuItem::with_id(
        app,
        MENU_LATER,
        route_labels[1].as_str(),
        true,
        Some("CmdOrCtrl+Shift+L"),
    )?;
    let notifications = MenuItem::with_id(
        app,
        MENU_NOTIFICATIONS,
        route_labels[2].as_str(),
        true,
        Some("CmdOrCtrl+Shift+N"),
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
            &separator,
            &build_item,
            &quit,
        ],
    )?;
    #[cfg(not(target_os = "linux"))]
    let items = TrayMenuItems {
        later,
        notifications,
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
    let dnd = MenuItem::with_id(
        app,
        MENU_DND_TOGGLE,
        route_labels[3].as_str(),
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
    format!("Synara — {unread} unread ({highlights} highlights), {later} later")
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
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let normalized_count = count.map(clamp_count).filter(|value| *value > 0);
        window.set_badge_count(normalized_count)?;

        #[cfg(target_os = "macos")]
        {
            window.set_badge_label(normalized_count.map(|value| value.to_string()))?;
        }
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
        .tooltip(&tray_tooltip(&initial_state))
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(handle_menu_event);

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
        eprintln!("failed to handle desktop menu event: {error}");
    }
}

pub(crate) fn tray_is_available<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.tray_by_id(TRAY_ICON_ID).is_some()
}

pub(crate) fn update_tray_state<R: Runtime>(
    app: AppHandle<R>,
    state: DesktopTrayState,
) -> Result<bool, String> {
    if tray_is_available(&app) {
        queue_tray_state_update(app, state)?;
        return Ok(true);
    }

    Ok(false)
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

        assert_eq!(on[3], "Do Not Disturb: On");
        assert_eq!(off[3], "Do Not Disturb: Off");
    }

    #[test]
    fn tray_dnd_toggle_dispatch_script_emits_custom_event() {
        assert_eq!(
            tray_dnd_toggle_dispatch_script(),
            "window.dispatchEvent(new CustomEvent('synara-tray-dnd-toggle'));"
        );
    }
}
