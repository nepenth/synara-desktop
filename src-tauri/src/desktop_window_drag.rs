//! Linux borderless window movement from the in-app title strip.
//!
//! Tauri's `plugin:window|start_dragging` returns from the button-press handler
//! before GTK runs, and tao then calls `gtk_window_begin_move_drag` with
//! `GDK_CURRENT_TIME`. On Wayland the compositor ignores that move. This module
//! calls `gtk_window_begin_move_drag` inside the WebKit button-press handler
//! with that event's button and timestamp, while the seat still holds the
//! button serial.

#[cfg(any(target_os = "linux", test))]
use serde::Deserialize;

/// GDK primary button. DOM `button === 0` is this same physical button.
const PRIMARY_BUTTON: u32 = 1;
/// `DesktopTitleBar` height at a 16px root (`toRem(40)`). Page zoom publishes
/// a live rectangle; this is only the stand-in before that message arrives.
const TITLE_STRIP_HEIGHT_PX: f64 = 40.0;
/// Right-edge stand-in for minimize, maximize, and close, including the
/// strip's right padding, up to the appearance zoom cap (150%).
const FALLBACK_CONTROLS_RESERVE_PX: f64 = 180.0;
const GDK_BUTTON_PRESS: i32 = 4;
const GDK_2BUTTON_PRESS: i32 = 5;
const DRAG_MESSAGE_HANDLER: &str = "synaraLinuxTitleDrag";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PressKind {
    Click,
    DoubleClick,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TitlePressAction {
    StartMove,
    ToggleMaximize,
    Ignore,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct CssRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl CssRect {
    fn contains(self, x: f64, y: f64) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.width && y < self.y + self.height
    }
}

#[derive(Clone, Debug, PartialEq)]
struct LiveTitleHitMap {
    inner_width: f64,
    drag: Vec<CssRect>,
    blocked: Vec<CssRect>,
}

struct TitlePress {
    button: u32,
    kind: PressKind,
    x: f64,
    y: f64,
    surface_width: f64,
    live: Option<LiveTitleHitMap>,
}

/// Linux window-state restores geometry and visibility, not decorations.
/// macOS and Windows keep `StateFlags::all()` via `Builder::default()`.
#[cfg(any(target_os = "linux", test))]
pub fn linux_window_state_flags() -> tauri_plugin_window_state::StateFlags {
    use tauri_plugin_window_state::StateFlags;

    StateFlags::SIZE
        | StateFlags::POSITION
        | StateFlags::MAXIMIZED
        | StateFlags::VISIBLE
        | StateFlags::FULLSCREEN
}

#[cfg(any(target_os = "linux", test))]
fn classify_title_press(press: &TitlePress) -> TitlePressAction {
    if press.button != PRIMARY_BUTTON {
        return TitlePressAction::Ignore;
    }

    let (x, y, drag, blocked) = hit_regions(press);
    if blocked.iter().any(|rect| rect.contains(x, y)) {
        return TitlePressAction::Ignore;
    }
    if !drag.iter().any(|rect| rect.contains(x, y)) {
        return TitlePressAction::Ignore;
    }

    match press.kind {
        PressKind::Click => TitlePressAction::StartMove,
        PressKind::DoubleClick => TitlePressAction::ToggleMaximize,
        PressKind::Other => TitlePressAction::Ignore,
    }
}

#[cfg(any(target_os = "linux", test))]
fn hit_regions(press: &TitlePress) -> (f64, f64, Vec<CssRect>, Vec<CssRect>) {
    if let Some(live) = press.live.as_ref().filter(|live| live_is_usable(live)) {
        let (x, y) = pointer_in_css_space(press.x, press.y, press.surface_width, live.inner_width);
        return (x, y, live.drag.clone(), live.blocked.clone());
    }

    let width = if press.surface_width.is_finite() {
        press.surface_width.max(0.0)
    } else {
        0.0
    };
    let reserve = FALLBACK_CONTROLS_RESERVE_PX.min(width);
    let drag = vec![CssRect {
        x: 0.0,
        y: 0.0,
        width,
        height: TITLE_STRIP_HEIGHT_PX,
    }];
    let blocked = vec![CssRect {
        x: width - reserve,
        y: 0.0,
        width: reserve,
        height: TITLE_STRIP_HEIGHT_PX,
    }];
    (press.x, press.y, drag, blocked)
}

#[cfg(any(target_os = "linux", test))]
fn live_is_usable(live: &LiveTitleHitMap) -> bool {
    live.inner_width.is_finite()
        && live.inner_width > 1.0
        && live
            .drag
            .iter()
            .any(|rect| rect.width > 0.0 && rect.height > 0.0)
}

#[cfg(any(target_os = "linux", test))]
fn pointer_in_css_space(x: f64, y: f64, surface_width: f64, inner_width: f64) -> (f64, f64) {
    let scale = surface_width / inner_width;
    if scale.is_finite() && (scale - 1.0).abs() > 0.05 {
        (x / scale, y / scale)
    } else {
        (x, y)
    }
}

#[cfg(any(target_os = "linux", test))]
fn parse_live_title_hit_map(json: &str) -> Option<LiveTitleHitMap> {
    let trimmed = json.trim();
    if trimmed.is_empty() || trimmed == "null" || trimmed == "undefined" {
        return None;
    }
    decode_hit_map(trimmed).or_else(|| {
        let inner = serde_json::from_str::<String>(trimmed).ok()?;
        decode_hit_map(inner.trim())
    })
}

#[cfg(any(target_os = "linux", test))]
fn decode_hit_map(json: &str) -> Option<LiveTitleHitMap> {
    let payload = serde_json::from_str::<HitPayload>(json).ok()?;
    Some(LiveTitleHitMap {
        inner_width: payload.inner_width,
        drag: finite_rects(payload.drag),
        blocked: finite_rects(payload.blocked),
    })
}

#[cfg(any(target_os = "linux", test))]
fn finite_rects(rects: Vec<RectPayload>) -> Vec<CssRect> {
    rects
        .into_iter()
        .filter_map(|rect| {
            let values = [rect.x, rect.y, rect.width, rect.height];
            if values.iter().all(|value| value.is_finite())
                && rect.width >= 0.0
                && rect.height >= 0.0
            {
                Some(CssRect {
                    x: rect.x,
                    y: rect.y,
                    width: rect.width,
                    height: rect.height,
                })
            } else {
                None
            }
        })
        .collect()
}

#[cfg(any(target_os = "linux", test))]
#[derive(Deserialize)]
struct HitPayload {
    #[serde(rename = "innerWidth", default)]
    inner_width: f64,
    #[serde(default)]
    drag: Vec<RectPayload>,
    #[serde(default)]
    blocked: Vec<RectPayload>,
}

#[cfg(any(target_os = "linux", test))]
#[derive(Deserialize)]
struct RectPayload {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

pub fn install<R: tauri::Runtime>(window: &tauri::WebviewWindow<R>) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        return window
            .with_webview(|webview| install_gtk_drag(webview.inner()))
            .map_err(|error| format!("Unable to install the Linux window drag hook: {error}"));
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = window;
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn install_gtk_drag(view: webkit2gtk::WebView) {
    use std::cell::RefCell;
    use std::rc::Rc;

    use webkit2gtk::glib::prelude::*;

    let hit = Rc::new(RefCell::new(None));
    install_hit_reporter(&view, Rc::clone(&hit));

    let weak = view.downgrade();
    view.connect_local("button-press-event", false, move |values| {
        let Some(view) = weak.upgrade() else {
            return Some(false.to_value());
        };
        Some(handle_button_press(&view, &hit, values).to_value())
    });
}

#[cfg(target_os = "linux")]
fn install_hit_reporter(
    view: &webkit2gtk::WebView,
    hit: std::rc::Rc<std::cell::RefCell<Option<LiveTitleHitMap>>>,
) {
    use std::rc::Rc;

    use webkit2gtk::{
        UserContentInjectedFrames, UserContentManagerExt, UserScript, UserScriptInjectionTime,
        WebViewExt,
    };

    let Some(manager) = view.user_content_manager() else {
        eprintln!(
            "[synara] WebKit user content manager is unavailable; title drag uses the fallback strip"
        );
        return;
    };

    let script = hit_reporter_script();
    manager.connect_script_message_received(Some(DRAG_MESSAGE_HANDLER), {
        let hit = Rc::clone(&hit);
        move |_manager, result| {
            if let Some(json) = javascript_result_json(result) {
                if let Some(map) = parse_live_title_hit_map(&json).filter(live_is_usable) {
                    *hit.borrow_mut() = Some(map);
                }
            }
        }
    });
    if !manager.register_script_message_handler(DRAG_MESSAGE_HANDLER) {
        eprintln!("[synara] Linux title-drag message handler was already registered");
    }
    manager.add_script(&UserScript::new(
        &script,
        UserContentInjectedFrames::TopFrame,
        UserScriptInjectionTime::End,
        &["*"],
        &[] as &[&str],
    ));

    let cancellable: Option<&webkit2gtk::gio::Cancellable> = None;
    view.evaluate_javascript(&script, None, None, cancellable, |_| {});
}

#[cfg(target_os = "linux")]
fn hit_reporter_script() -> String {
    format!(
        r#"
(() => {{
  if (window.__SYNARA_LINUX_TITLE_DRAG__) return;
  window.__SYNARA_LINUX_TITLE_DRAG__ = true;
  const channelName = "{handler}";
  let frame = 0;
  const boxes = (selector) => Array.prototype.map.call(document.querySelectorAll(selector), (el) => {{
    const rect = el.getBoundingClientRect();
    return {{ x: rect.x, y: rect.y, width: rect.width, height: rect.height }};
  }});
  const publish = () => {{
    frame = 0;
    try {{
      const channel = window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers[channelName];
      if (!channel || typeof channel.postMessage !== "function") return;
      channel.postMessage({{
        innerWidth: window.innerWidth,
        drag: boxes("[data-synara-window-drag]"),
        blocked: boxes("[data-synara-window-controls]")
      }});
    }} catch (error) {{
      console.error("[synara] title drag geometry", error);
    }}
  }};
  const schedule = () => {{
    if (frame) return;
    frame = requestAnimationFrame(publish);
  }};
  window.addEventListener("resize", schedule);
  if (document.documentElement && typeof MutationObserver === "function") {{
    new MutationObserver(schedule).observe(document.documentElement, {{
      subtree: true,
      childList: true,
      attributes: true,
      attributeFilter: ["class", "style", "data-synara-window-drag", "data-synara-window-controls"]
    }});
  }}
  schedule();
}})();
"#,
        handler = DRAG_MESSAGE_HANDLER
    )
}

#[cfg(target_os = "linux")]
fn handle_button_press(
    view: &webkit2gtk::WebView,
    hit: &std::rc::Rc<std::cell::RefCell<Option<LiveTitleHitMap>>>,
    values: &[webkit2gtk::glib::Value],
) -> bool {
    let Some(event) = values.get(1).map(event_from_signal_value) else {
        return false;
    };
    if event.is_null() {
        return false;
    }

    let Some(button) = event_button(event) else {
        return false;
    };
    let Some((x, y)) = event_coords(event) else {
        return false;
    };
    let action = classify_title_press(&TitlePress {
        button,
        kind: press_kind_from_gdk(event_type(event)),
        x,
        y,
        surface_width: allocated_width(view),
        live: hit.borrow().clone(),
    });

    match action {
        TitlePressAction::Ignore => false,
        TitlePressAction::StartMove => begin_move_from_event(view, event, button),
        TitlePressAction::ToggleMaximize => toggle_maximized(view),
    }
}

#[cfg(target_os = "linux")]
fn press_kind_from_gdk(event_type: i32) -> PressKind {
    match event_type {
        GDK_BUTTON_PRESS => PressKind::Click,
        GDK_2BUTTON_PRESS => PressKind::DoubleClick,
        _ => PressKind::Other,
    }
}

#[cfg(target_os = "linux")]
fn begin_move_from_event(
    view: &webkit2gtk::WebView,
    event: *mut std::ffi::c_void,
    button: u32,
) -> bool {
    let Some((root_x, root_y)) = event_root(event) else {
        return false;
    };
    let Some(window) = gtk_window(view) else {
        return false;
    };
    // GTK 3's Wayland backend ignores this timestamp and sends the seat's
    // implicit-grab serial captured for the button event now being dispatched.
    // That serial is valid for `xdg_toplevel_move` only inside this handler.
    // The event time is still passed through: on X11 it is the real timestamp,
    // and `GDK_CURRENT_TIME` (0) is what the queued tao drag uses.
    unsafe {
        gtk_window_begin_move_drag(
            window,
            button as std::os::raw::c_int,
            root_x.round() as std::os::raw::c_int,
            root_y.round() as std::os::raw::c_int,
            gdk_event_get_time(event),
        );
    }
    true
}

#[cfg(target_os = "linux")]
fn toggle_maximized(view: &webkit2gtk::WebView) -> bool {
    let Some(window) = gtk_window(view) else {
        return false;
    };
    unsafe {
        if gtk_window_is_maximized(window) != 0 {
            gtk_window_unmaximize(window);
        } else {
            gtk_window_maximize(window);
        }
    }
    true
}

#[cfg(target_os = "linux")]
fn gtk_window(view: &webkit2gtk::WebView) -> Option<*mut std::ffi::c_void> {
    let widget = widget_ptr(view);
    if widget.is_null() {
        return None;
    }
    let top = unsafe { gtk_widget_get_toplevel(widget) };
    if top.is_null() || !widget_is_window(top) {
        return None;
    }
    Some(top)
}

#[cfg(target_os = "linux")]
fn widget_is_window(widget: *mut std::ffi::c_void) -> bool {
    unsafe {
        gtk_widget_is_toplevel(widget) != 0
            && webkit2gtk::glib::gobject_ffi::g_type_check_instance_is_a(
                widget.cast::<webkit2gtk::glib::gobject_ffi::GTypeInstance>(),
                gtk_window_get_type(),
            ) != 0
    }
}

#[cfg(target_os = "linux")]
fn widget_ptr(view: &webkit2gtk::WebView) -> *mut std::ffi::c_void {
    use webkit2gtk::glib::translate::ToGlibPtr;
    <webkit2gtk::WebView as ToGlibPtr<*mut webkit2gtk::ffi::WebKitWebView>>::to_glib_none(view)
        .0
        .cast()
}

#[cfg(target_os = "linux")]
fn allocated_width(view: &webkit2gtk::WebView) -> f64 {
    let widget = widget_ptr(view);
    if widget.is_null() {
        0.0
    } else {
        unsafe { gtk_widget_get_allocated_width(widget) as f64 }
    }
}

#[cfg(target_os = "linux")]
fn event_from_signal_value(value: &webkit2gtk::glib::Value) -> *mut std::ffi::c_void {
    use webkit2gtk::glib::translate::ToGlibPtr;
    unsafe {
        let stash = value.to_glib_none();
        webkit2gtk::glib::gobject_ffi::g_value_get_boxed(stash.0).cast()
    }
}

#[cfg(target_os = "linux")]
fn event_type(event: *mut std::ffi::c_void) -> i32 {
    unsafe { gdk_event_get_event_type(event) as i32 }
}

#[cfg(target_os = "linux")]
fn event_button(event: *mut std::ffi::c_void) -> Option<u32> {
    let mut button = 0u32;
    let ok = unsafe { gdk_event_get_button(event, &mut button) };
    (ok != 0).then_some(button)
}

#[cfg(target_os = "linux")]
fn event_coords(event: *mut std::ffi::c_void) -> Option<(f64, f64)> {
    let mut x = 0.0;
    let mut y = 0.0;
    let ok = unsafe { gdk_event_get_coords(event, &mut x, &mut y) };
    (ok != 0).then_some((x, y))
}

#[cfg(target_os = "linux")]
fn event_root(event: *mut std::ffi::c_void) -> Option<(f64, f64)> {
    let mut x = 0.0;
    let mut y = 0.0;
    let ok = unsafe { gdk_event_get_root_coords(event, &mut x, &mut y) };
    (ok != 0).then_some((x, y))
}

#[cfg(target_os = "linux")]
fn javascript_result_json(result: &webkit2gtk::JavascriptResult) -> Option<String> {
    use webkit2gtk::glib::translate::ToGlibPtr;
    unsafe {
        let value = webkit_javascript_result_get_js_value(
            <webkit2gtk::JavascriptResult as ToGlibPtr<
                *mut webkit2gtk::ffi::WebKitJavascriptResult,
            >>::to_glib_none(result)
            .0
            .cast(),
        );
        if value.is_null() {
            return None;
        }
        let json = jsc_value_to_json(value, 0);
        if json.is_null() {
            return None;
        }
        let text = std::ffi::CStr::from_ptr(json)
            .to_string_lossy()
            .into_owned();
        webkit2gtk::glib::ffi::g_free(json.cast());
        Some(text)
    }
}

#[cfg(target_os = "linux")]
#[link(name = "gtk-3")]
#[link(name = "gdk-3")]
#[link(name = "javascriptcoregtk-4.1")]
#[link(name = "webkit2gtk-4.1")]
unsafe extern "C" {
    fn gtk_window_get_type() -> usize;
    fn gtk_widget_get_toplevel(widget: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    fn gtk_widget_is_toplevel(widget: *mut std::ffi::c_void) -> std::os::raw::c_int;
    fn gtk_widget_get_allocated_width(widget: *mut std::ffi::c_void) -> std::os::raw::c_int;
    fn gtk_window_begin_move_drag(
        window: *mut std::ffi::c_void,
        button: std::os::raw::c_int,
        root_x: std::os::raw::c_int,
        root_y: std::os::raw::c_int,
        timestamp: u32,
    );
    fn gtk_window_is_maximized(window: *mut std::ffi::c_void) -> std::os::raw::c_int;
    fn gtk_window_maximize(window: *mut std::ffi::c_void);
    fn gtk_window_unmaximize(window: *mut std::ffi::c_void);
    fn gdk_event_get_event_type(event: *mut std::ffi::c_void) -> std::os::raw::c_int;
    fn gdk_event_get_time(event: *mut std::ffi::c_void) -> u32;
    fn gdk_event_get_button(event: *mut std::ffi::c_void, button: *mut u32) -> std::os::raw::c_int;
    fn gdk_event_get_coords(
        event: *mut std::ffi::c_void,
        x: *mut f64,
        y: *mut f64,
    ) -> std::os::raw::c_int;
    fn gdk_event_get_root_coords(
        event: *mut std::ffi::c_void,
        x: *mut f64,
        y: *mut f64,
    ) -> std::os::raw::c_int;
    fn webkit_javascript_result_get_js_value(
        result: *mut std::ffi::c_void,
    ) -> *mut std::ffi::c_void;
    fn jsc_value_to_json(value: *mut std::ffi::c_void, indent: u32) -> *mut std::os::raw::c_char;
}

#[cfg(test)]
mod tests {
    use super::{
        classify_title_press, linux_window_state_flags, parse_live_title_hit_map, CssRect,
        LiveTitleHitMap, PressKind, TitlePress, TitlePressAction, PRIMARY_BUTTON,
    };
    use tauri_plugin_window_state::StateFlags;

    fn press(kind: PressKind, x: f64, y: f64, live: Option<LiveTitleHitMap>) -> TitlePress {
        TitlePress {
            button: PRIMARY_BUTTON,
            kind,
            x,
            y,
            surface_width: 1280.0,
            live,
        }
    }

    fn sample_map() -> LiveTitleHitMap {
        LiveTitleHitMap {
            inner_width: 1280.0,
            drag: vec![CssRect {
                x: 0.0,
                y: 0.0,
                width: 1280.0,
                height: 40.0,
            }],
            blocked: vec![CssRect {
                x: 1080.0,
                y: 4.0,
                width: 180.0,
                height: 32.0,
            }],
        }
    }

    #[test]
    fn linux_window_state_keeps_geometry_and_drops_decorations() {
        let flags = linux_window_state_flags();
        let kept = StateFlags::SIZE
            | StateFlags::POSITION
            | StateFlags::MAXIMIZED
            | StateFlags::VISIBLE
            | StateFlags::FULLSCREEN;
        assert_eq!(flags.bits(), kept.bits());
        assert!(flags.contains(StateFlags::SIZE));
        assert!(flags.contains(StateFlags::POSITION));
        assert!(flags.contains(StateFlags::MAXIMIZED));
        assert!(flags.contains(StateFlags::VISIBLE));
        assert!(flags.contains(StateFlags::FULLSCREEN));
        assert!(!flags.contains(StateFlags::DECORATIONS));
        assert_ne!(flags.bits(), StateFlags::all().bits());
    }

    #[test]
    fn primary_press_on_the_drag_surface_starts_a_move() {
        let map = sample_map();
        assert_eq!(
            classify_title_press(&press(PressKind::Click, 40.0, 12.0, Some(map.clone()))),
            TitlePressAction::StartMove
        );
        assert_eq!(
            classify_title_press(&press(PressKind::Click, 12.0, 2.0, Some(map))),
            TitlePressAction::StartMove
        );
    }

    #[test]
    fn press_on_window_controls_does_not_start_a_move() {
        let map = sample_map();
        assert_eq!(
            classify_title_press(&press(PressKind::Click, 1100.0, 16.0, Some(map.clone()))),
            TitlePressAction::Ignore
        );
        assert_eq!(
            classify_title_press(&press(
                PressKind::DoubleClick,
                1200.0,
                20.0,
                Some(map.clone())
            )),
            TitlePressAction::Ignore
        );
        let mut other = press(PressKind::Click, 40.0, 12.0, Some(map));
        other.button = 3;
        assert_eq!(classify_title_press(&other), TitlePressAction::Ignore);
    }

    #[test]
    fn double_click_on_the_drag_surface_toggles_maximize_without_a_second_move() {
        let map = sample_map();
        assert_eq!(
            classify_title_press(&press(
                PressKind::DoubleClick,
                80.0,
                10.0,
                Some(map.clone())
            )),
            TitlePressAction::ToggleMaximize
        );
        assert_ne!(
            classify_title_press(&press(
                PressKind::DoubleClick,
                80.0,
                10.0,
                Some(map.clone())
            )),
            TitlePressAction::StartMove
        );
        assert_eq!(
            classify_title_press(&press(PressKind::Other, 80.0, 10.0, Some(map))),
            TitlePressAction::Ignore
        );
    }

    #[test]
    fn content_below_the_strip_and_the_fallback_reserve_are_not_drag_surfaces() {
        assert_eq!(
            classify_title_press(&press(PressKind::Click, 40.0, 80.0, None)),
            TitlePressAction::Ignore
        );
        assert_eq!(
            classify_title_press(&press(PressKind::Click, 20.0, 8.0, None)),
            TitlePressAction::StartMove
        );
        assert_eq!(
            classify_title_press(&press(PressKind::Click, 1200.0, 8.0, None)),
            TitlePressAction::Ignore
        );
        assert_eq!(
            classify_title_press(&press(PressKind::DoubleClick, 20.0, 8.0, None)),
            TitlePressAction::ToggleMaximize
        );
    }

    #[test]
    fn live_rectangles_scale_into_css_pixels_before_the_hit_test() {
        let live = LiveTitleHitMap {
            inner_width: 1000.0,
            drag: vec![CssRect {
                x: 0.0,
                y: 0.0,
                width: 1000.0,
                height: 40.0,
            }],
            blocked: vec![CssRect {
                x: 860.0,
                y: 0.0,
                width: 140.0,
                height: 40.0,
            }],
        };
        let mut device = press(PressKind::Click, 100.0, 10.0, Some(live.clone()));
        device.surface_width = 2000.0;
        assert_eq!(classify_title_press(&device), TitlePressAction::StartMove);

        device.x = 1800.0;
        assert_eq!(classify_title_press(&device), TitlePressAction::Ignore);
    }

    #[test]
    fn parses_title_strip_rectangles_and_ignores_garbage() {
        let map = parse_live_title_hit_map(
            r#"{"innerWidth":900,"drag":[{"x":0,"y":0,"width":700,"height":40}],"blocked":[{"x":700,"y":4,"width":180,"height":32}]}"#,
        )
        .expect("geometry payload");
        assert_eq!(map.inner_width, 900.0);
        assert_eq!(map.drag.len(), 1);
        assert_eq!(map.blocked[0].x, 700.0);
        assert!(parse_live_title_hit_map("not json").is_none());
        assert!(parse_live_title_hit_map(
            r#"{"innerWidth":1,"drag":[{"x":0,"y":0,"width":null,"height":40}]}"#
        )
        .is_none());
    }
}
