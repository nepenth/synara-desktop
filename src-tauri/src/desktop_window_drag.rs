//! Linux borderless window movement from the in-app title strip.
//!
//! Tauri's `plugin:window|start_dragging` returns from the button-press handler
//! before GTK runs, and tao then calls `gtk_window_begin_move_drag` with
//! `GDK_CURRENT_TIME`. On Wayland the compositor ignores that move. This module
//! owns the strip's presses natively instead, the way GTK's own title bars do:
//! a primary press on the drag surface is held back from WebKit, and once the
//! pointer passes the GTK drag threshold while the button is still down,
//! `gtk_window_begin_move_drag` runs from that motion event. GTK 3's Wayland
//! backend sends the seat's implicit-grab serial from the press, which stays
//! valid while the button is held. A double-click toggles maximize and never
//! starts a move, so double-click has a single owner.
//!
//! The page publishes the strip geometry over a WebKit script message. Until a
//! valid map arrives, nothing is a drag surface: presses go to the page.

#[cfg(any(target_os = "linux", test))]
use serde::Deserialize;

/// GDK primary button. DOM `button === 0` is this same physical button.
#[cfg(any(target_os = "linux", test))]
const PRIMARY_BUTTON: u32 = 1;
/// The strip is `toRem(40)`: 30–60 CSS px across the 75–150% appearance zoom.
/// A drag rectangle taller than this, or reaching below the top band, is not
/// the strip and the whole map is rejected.
#[cfg(any(target_os = "linux", test))]
const MAX_DRAG_RECT_HEIGHT_CSS: f64 = 80.0;
#[cfg(any(target_os = "linux", test))]
const MAX_DRAG_RECT_BOTTOM_CSS: f64 = 120.0;
/// GTK's default `gtk-dnd-drag-threshold`, used if the setting is unreadable.
#[cfg(any(target_os = "linux", test))]
const DEFAULT_DRAG_THRESHOLD_PX: f64 = 8.0;
#[cfg(target_os = "linux")]
const GDK_BUTTON_PRESS: i32 = 4;
#[cfg(target_os = "linux")]
const GDK_2BUTTON_PRESS: i32 = 5;
#[cfg(target_os = "linux")]
const DRAG_MESSAGE_HANDLER: &str = "synaraLinuxTitleDrag";

/// Opt-in trace for diagnosing the title-strip drag on a live session:
/// `SYNARA_DEBUG_DRAG=/path/to/file synara`. Off when the variable is unset.
#[cfg(target_os = "linux")]
fn drag_trace(message: impl FnOnce() -> String) {
    use std::io::Write;
    let Some(path) = std::env::var_os("SYNARA_DEBUG_DRAG") else {
        return;
    };
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{}", message());
    }
}

#[cfg(any(target_os = "linux", test))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PressKind {
    Click,
    DoubleClick,
    Other,
}

#[cfg(any(target_os = "linux", test))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TitlePressAction {
    /// Hold the press back from WebKit and move once the pointer travels.
    ArmMove,
    ToggleMaximize,
    Ignore,
}

#[cfg(any(target_os = "linux", test))]
#[derive(Clone, Copy, Debug, PartialEq)]
struct CssRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[cfg(any(target_os = "linux", test))]
impl CssRect {
    fn contains(self, x: f64, y: f64) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.width && y < self.y + self.height
    }
}

#[cfg(any(target_os = "linux", test))]
#[derive(Clone, Debug, PartialEq)]
struct LiveTitleHitMap {
    inner_width: f64,
    drag: Vec<CssRect>,
    blocked: Vec<CssRect>,
    /// Something the page layered over the strip (dialog, viewer, menu,
    /// tooltip) receives pointer events there. No press may move the window.
    overlay: bool,
}

#[cfg(any(target_os = "linux", test))]
#[derive(Debug)]
struct TitlePress {
    button: u32,
    kind: PressKind,
    x: f64,
    y: f64,
    surface_width: f64,
    zoom: f64,
    live: Option<LiveTitleHitMap>,
}

/// One script message after the token check.
#[cfg(any(target_os = "linux", test))]
#[derive(Debug, PartialEq)]
enum HitMessage {
    /// Not ours (wrong or missing token) or unreadable: keep the current map.
    Ignore,
    /// From the page: replace the map. `None` clears it (strip unmounted or
    /// geometry outside the strip's bounds).
    Replace(Option<LiveTitleHitMap>),
}

#[cfg(any(target_os = "linux", test))]
#[derive(Clone, Copy, Debug, PartialEq)]
struct PendingMove {
    button: u32,
    root_x: f64,
    root_y: f64,
}

#[cfg(any(target_os = "linux", test))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PressStep {
    /// Let WebKit handle the press.
    Pass,
    /// Swallow the press; a later motion may start the move.
    Consume,
    ToggleMaximize,
}

#[cfg(any(target_os = "linux", test))]
#[derive(Clone, Copy, Debug, PartialEq)]
enum MotionStep {
    Pass,
    /// Still under the threshold; keep the motion from WebKit.
    Wait,
    BeginMove(PendingMove),
}

/// Press → threshold motion → move, as GTK title bars do it.
#[cfg(any(target_os = "linux", test))]
#[derive(Debug, Default)]
struct TitleGesture {
    pending: Option<PendingMove>,
}

#[cfg(any(target_os = "linux", test))]
impl TitleGesture {
    fn press(
        &mut self,
        action: TitlePressAction,
        button: u32,
        root_x: f64,
        root_y: f64,
    ) -> PressStep {
        match action {
            TitlePressAction::ArmMove => {
                self.pending = Some(PendingMove {
                    button,
                    root_x,
                    root_y,
                });
                PressStep::Consume
            }
            TitlePressAction::ToggleMaximize => {
                self.pending = None;
                PressStep::ToggleMaximize
            }
            TitlePressAction::Ignore => {
                self.pending = None;
                PressStep::Pass
            }
        }
    }

    fn motion(
        &mut self,
        root_x: f64,
        root_y: f64,
        button_held: bool,
        threshold: f64,
    ) -> MotionStep {
        let Some(pending) = self.pending else {
            return MotionStep::Pass;
        };
        if !button_held {
            // The release went elsewhere (grab broken, focus change).
            self.pending = None;
            return MotionStep::Pass;
        }
        if passes_drag_threshold(pending.root_x, pending.root_y, root_x, root_y, threshold) {
            self.pending = None;
            MotionStep::BeginMove(pending)
        } else {
            MotionStep::Wait
        }
    }

    /// True when the matching press was swallowed, so WebKit must not see a
    /// lone release either.
    fn release(&mut self) -> bool {
        self.pending.take().is_some()
    }
}

/// `gtk_drag_check_threshold`: either axis strictly beyond the threshold.
#[cfg(any(target_os = "linux", test))]
fn passes_drag_threshold(start_x: f64, start_y: f64, x: f64, y: f64, threshold: f64) -> bool {
    let threshold = if threshold.is_finite() && threshold >= 0.0 {
        threshold
    } else {
        DEFAULT_DRAG_THRESHOLD_PX
    };
    (x - start_x).abs() > threshold || (y - start_y).abs() > threshold
}

/// GDK `GDK_BUTTON1_MASK` is `1 << 8`; buttons 1–5 are consecutive.
#[cfg(any(target_os = "linux", test))]
fn button_mask(button: u32) -> u32 {
    if (1..=5).contains(&button) {
        1 << (7 + button)
    } else {
        0
    }
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
    // No published geometry means nothing is a drag surface.
    let Some(live) = press.live.as_ref() else {
        return TitlePressAction::Ignore;
    };
    if live.overlay {
        return TitlePressAction::Ignore;
    }
    let Some((x, y)) = pointer_in_css_space(
        press.x,
        press.y,
        press.surface_width,
        live.inner_width,
        press.zoom,
    ) else {
        return TitlePressAction::Ignore;
    };
    if live.blocked.iter().any(|rect| rect.contains(x, y)) {
        return TitlePressAction::Ignore;
    }
    if !live.drag.iter().any(|rect| rect.contains(x, y)) {
        return TitlePressAction::Ignore;
    }

    match press.kind {
        PressKind::Click => TitlePressAction::ArmMove,
        PressKind::DoubleClick => TitlePressAction::ToggleMaximize,
        PressKind::Other => TitlePressAction::Ignore,
    }
}

/// Widget pixels to CSS pixels. Appearance zoom scales the root font size, so
/// the strip's own rectangles already carry it; WebKit page zoom (normally
/// 1.0) is the only widget↔CSS factor. `None` when the published width does
/// not match the widget: the map predates a resize, so the press is not
/// classified until the page republishes.
#[cfg(any(target_os = "linux", test))]
fn pointer_in_css_space(
    x: f64,
    y: f64,
    surface_width: f64,
    inner_width: f64,
    zoom: f64,
) -> Option<(f64, f64)> {
    let zoom = if zoom.is_finite() && zoom > 0.0 {
        zoom
    } else {
        1.0
    };
    if !surface_width.is_finite() || !inner_width.is_finite() || inner_width <= 1.0 {
        return None;
    }
    let tolerance = (surface_width * 0.01).max(2.0);
    if (inner_width * zoom - surface_width).abs() > tolerance {
        return None;
    }
    Some((x / zoom, y / zoom))
}

#[cfg(any(target_os = "linux", test))]
fn parse_hit_message(json: &str, token: &str) -> HitMessage {
    let trimmed = json.trim();
    let payload = serde_json::from_str::<HitPayload>(trimmed)
        .ok()
        .or_else(|| {
            let inner = serde_json::from_str::<String>(trimmed).ok()?;
            serde_json::from_str::<HitPayload>(inner.trim()).ok()
        });
    let Some(payload) = payload else {
        return HitMessage::Ignore;
    };
    if token.is_empty() || payload.token.as_deref() != Some(token) {
        return HitMessage::Ignore;
    }
    HitMessage::Replace(validated_hit_map(payload))
}

#[cfg(any(target_os = "linux", test))]
fn validated_hit_map(payload: HitPayload) -> Option<LiveTitleHitMap> {
    if !payload.inner_width.is_finite() || payload.inner_width <= 1.0 {
        return None;
    }
    let drag: Vec<CssRect> = finite_rects(payload.drag?)?
        .into_iter()
        .filter(|rect| rect.width > 0.0 && rect.height > 0.0)
        .collect();
    if drag.is_empty() {
        return None;
    }
    let strip_shaped = drag.iter().all(|rect| {
        rect.y >= -1.0
            && rect.height <= MAX_DRAG_RECT_HEIGHT_CSS
            && rect.y + rect.height <= MAX_DRAG_RECT_BOTTOM_CSS
    });
    if !strip_shaped {
        return None;
    }
    Some(LiveTitleHitMap {
        inner_width: payload.inner_width,
        drag,
        blocked: finite_rects(payload.blocked.unwrap_or_default())?,
        // A missing flag is not proof that nothing covers the strip.
        overlay: payload.overlay.unwrap_or(true),
    })
}

/// Any non-finite or negative-size rectangle rejects the list.
#[cfg(any(target_os = "linux", test))]
fn finite_rects(rects: Vec<RectPayload>) -> Option<Vec<CssRect>> {
    rects
        .into_iter()
        .map(|rect| {
            let values = [rect.x?, rect.y?, rect.width?, rect.height?];
            if values.iter().all(|value| value.is_finite()) && values[2] >= 0.0 && values[3] >= 0.0
            {
                Some(CssRect {
                    x: values[0],
                    y: values[1],
                    width: values[2],
                    height: values[3],
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
    #[serde(default)]
    token: Option<String>,
    #[serde(rename = "innerWidth", default)]
    inner_width: f64,
    #[serde(default)]
    drag: Option<Vec<RectPayload>>,
    #[serde(default)]
    blocked: Option<Vec<RectPayload>>,
    #[serde(default)]
    overlay: Option<bool>,
}

#[cfg(any(target_os = "linux", test))]
#[derive(Deserialize)]
struct RectPayload {
    x: Option<f64>,
    y: Option<f64>,
    width: Option<f64>,
    height: Option<f64>,
}

pub fn install<R: tauri::Runtime>(window: &tauri::WebviewWindow<R>) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    let result = window
        .with_webview(|webview| install_gtk_drag(webview.inner()))
        .map_err(|error| format!("Unable to install the Linux window drag hook: {error}"));
    #[cfg(not(target_os = "linux"))]
    let result = {
        let _ = window;
        Ok(())
    };
    result
}

#[cfg(target_os = "linux")]
type SharedHitMap = std::rc::Rc<std::cell::RefCell<Option<LiveTitleHitMap>>>;
#[cfg(target_os = "linux")]
type SharedGesture = std::rc::Rc<std::cell::RefCell<TitleGesture>>;

#[cfg(target_os = "linux")]
fn install_gtk_drag(view: webkit2gtk::WebView) {
    use std::cell::RefCell;
    use std::rc::Rc;

    use webkit2gtk::glib::prelude::*;

    let hit: SharedHitMap = Rc::new(RefCell::new(None));
    let reporter = install_hit_reporter(&view, Rc::clone(&hit));
    drag_trace(|| format!("install: hit reporter installed={reporter}"));
    if !reporter {
        // Without geometry from the page nothing is a drag surface. Leave
        // every press to WebKit rather than guess where the strip is.
        return;
    }
    let gesture: SharedGesture = Rc::new(RefCell::new(TitleGesture::default()));

    let weak = view.downgrade();
    let press_gesture = Rc::clone(&gesture);
    view.connect_local("button-press-event", false, move |values| {
        let Some(view) = weak.upgrade() else {
            return Some(false.to_value());
        };
        Some(handle_button_press(&view, &hit, &press_gesture, values).to_value())
    });

    let weak = view.downgrade();
    let motion_gesture = Rc::clone(&gesture);
    view.connect_local("motion-notify-event", false, move |values| {
        let Some(view) = weak.upgrade() else {
            return Some(false.to_value());
        };
        Some(handle_motion(&view, &motion_gesture, values).to_value())
    });

    view.connect_local("button-release-event", false, move |_values| {
        Some(gesture.borrow_mut().release().to_value())
    });
}

/// Registers the geometry channel. False when the page cannot publish, in
/// which case no press hook is installed.
#[cfg(target_os = "linux")]
fn install_hit_reporter(view: &webkit2gtk::WebView, hit: SharedHitMap) -> bool {
    use std::rc::Rc;

    use webkit2gtk::{
        LoadEvent, UserContentInjectedFrames, UserContentManagerExt, UserScript,
        UserScriptInjectionTime, WebViewExt,
    };

    let Some(manager) = view.user_content_manager() else {
        eprintln!("[synara] WebKit user content manager is unavailable; Linux title drag is off");
        return false;
    };

    // Script message handlers are visible to every frame, including widget
    // iframes. Only the top-frame script holds this token, in a closure.
    let token = message_token();
    let script = hit_reporter_script(&token);
    if !manager.register_script_message_handler(DRAG_MESSAGE_HANDLER) {
        eprintln!("[synara] Linux title-drag message handler did not register; title drag is off");
        return false;
    }
    manager.connect_script_message_received(Some(DRAG_MESSAGE_HANDLER), {
        let hit = Rc::clone(&hit);
        move |_manager, result| {
            let Some(json) = javascript_result_json(result) else {
                drag_trace(|| "message: unreadable script result".to_owned());
                return;
            };
            let parsed = parse_hit_message(&json, &token);
            drag_trace(|| format!("message: {json} -> {parsed:?}"));
            if let HitMessage::Replace(map) = parsed {
                *hit.borrow_mut() = map;
            }
        }
    });
    // A new document has not published yet; the old map no longer applies.
    // The user script only reaches loads that start after it is added, and
    // the app's first document is already loading by the time this hook is
    // installed, so evaluate the reporter again whenever a load finishes. The
    // script's own guard keeps it to one instance per document.
    view.connect_load_changed({
        let hit = Rc::clone(&hit);
        let script = script.clone();
        move |view, event| match event {
            LoadEvent::Committed => *hit.borrow_mut() = None,
            LoadEvent::Finished => {
                let cancellable: Option<&webkit2gtk::gio::Cancellable> = None;
                view.evaluate_javascript(&script, None, None, cancellable, |_| {});
            }
            _ => {}
        }
    });
    manager.add_script(&UserScript::new(
        &script,
        UserContentInjectedFrames::TopFrame,
        UserScriptInjectionTime::End,
        &["*"],
        &[] as &[&str],
    ));

    let cancellable: Option<&webkit2gtk::gio::Cancellable> = None;
    view.evaluate_javascript(&script, None, None, cancellable, |_| {});
    true
}

/// 128 bits from two independently OS-seeded SipHash keys. This is a
/// same-process frame check, not a secret that leaves the process.
#[cfg(target_os = "linux")]
fn message_token() -> String {
    use std::hash::{BuildHasher, Hasher};

    let mut parts = [0u64; 2];
    for (index, part) in parts.iter_mut().enumerate() {
        let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
        hasher.write_usize(index);
        hasher.write_u128(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos())
                .unwrap_or_default(),
        );
        *part = hasher.finish();
    }
    format!("{:016x}{:016x}", parts[0], parts[1])
}

#[cfg(target_os = "linux")]
fn hit_reporter_script(token: &str) -> String {
    let token_json = serde_json::to_string(token).unwrap_or_else(|_| "\"\"".to_owned());
    format!(
        r#"
(() => {{
  if (window.top !== window || window.__SYNARA_LINUX_TITLE_DRAG__) return;
  window.__SYNARA_LINUX_TITLE_DRAG__ = true;
  const token = {token_json};
  const channelName = "{handler}";
  const owner = "[data-synara-window-drag],[data-synara-window-controls]";
  let frame = 0;
  let last = "";
  const boxes = (selector) => Array.prototype.map.call(document.querySelectorAll(selector), (el) => {{
    const rect = el.getBoundingClientRect();
    return {{ x: rect.x, y: rect.y, width: rect.width, height: rect.height }};
  }});
  // True when anything other than the strip receives pointer events over it:
  // a dialog or viewer backdrop, a menu, or a tooltip from #portalContainer.
  const covered = (rects) => {{
    for (const rect of rects) {{
      if (!(rect.width > 0 && rect.height > 0)) continue;
      const rows = [rect.y + 2, rect.y + rect.height / 2, rect.y + rect.height - 2];
      const right = rect.x + rect.width - 1;
      for (let x = rect.x + 1; ; x = Math.min(x + 20, right)) {{
        for (const y of rows) {{
          if (x < 0 || y < 0 || x >= window.innerWidth || y >= window.innerHeight) continue;
          const hit = document.elementFromPoint(x, y);
          if (hit && !(hit.closest && hit.closest(owner))) return true;
        }}
        if (x >= right) break;
      }}
    }}
    return false;
  }};
  const publish = () => {{
    frame = 0;
    try {{
      const channel = window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers[channelName];
      if (!channel || typeof channel.postMessage !== "function") return;
      const drag = boxes("[data-synara-window-drag]");
      const message = {{
        token,
        innerWidth: window.innerWidth,
        drag,
        blocked: boxes("[data-synara-window-controls]"),
        overlay: covered(drag)
      }};
      const serialized = JSON.stringify(message);
      if (serialized === last) return;
      last = serialized;
      channel.postMessage(message);
    }} catch (error) {{
      console.error("[synara] title drag geometry", error);
    }}
  }};
  const schedule = () => {{
    if (frame) return;
    frame = requestAnimationFrame(publish);
  }};
  window.addEventListener("resize", schedule);
  // Overlays fade or slide in after they mount; republish when they settle.
  document.addEventListener("transitionend", schedule, true);
  document.addEventListener("animationend", schedule, true);
  if (document.documentElement && typeof MutationObserver === "function") {{
    new MutationObserver(schedule).observe(document.documentElement, {{
      subtree: true,
      childList: true,
      attributes: true,
      attributeFilter: ["class", "style", "hidden", "data-synara-window-drag", "data-synara-window-controls"]
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
    hit: &SharedHitMap,
    gesture: &SharedGesture,
    values: &[webkit2gtk::glib::Value],
) -> bool {
    use webkit2gtk::WebViewExt;

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
    let Some((root_x, root_y)) = event_root(event) else {
        return false;
    };
    let press = TitlePress {
        button,
        kind: press_kind_from_gdk(event_type(event)),
        x,
        y,
        surface_width: allocated_width(view),
        zoom: view.zoom_level(),
        live: hit.borrow().clone(),
    };
    let action = classify_title_press(&press);

    let step = gesture.borrow_mut().press(action, button, root_x, root_y);
    drag_trace(|| format!("press: {press:?} root=({root_x},{root_y}) -> {action:?} / {step:?}"));
    match step {
        PressStep::Pass => false,
        PressStep::Consume => true,
        PressStep::ToggleMaximize => toggle_maximized(view),
    }
}

#[cfg(target_os = "linux")]
fn handle_motion(
    view: &webkit2gtk::WebView,
    gesture: &SharedGesture,
    values: &[webkit2gtk::glib::Value],
) -> bool {
    let pending = gesture.borrow().pending;
    let Some(pending) = pending else {
        return false;
    };
    let Some(event) = values.get(1).map(event_from_signal_value) else {
        return false;
    };
    if event.is_null() {
        return false;
    }
    let Some((root_x, root_y)) = event_root(event) else {
        return false;
    };
    let held = event_state(event).is_some_and(|state| state & button_mask(pending.button) != 0);

    let step = gesture
        .borrow_mut()
        .motion(root_x, root_y, held, drag_threshold());
    drag_trace(|| format!("motion: root=({root_x},{root_y}) held={held} -> {step:?}"));
    match step {
        MotionStep::Pass => false,
        MotionStep::Wait => true,
        MotionStep::BeginMove(start) => {
            begin_move(view, start, unsafe { gdk_event_get_time(event) })
        }
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
fn begin_move(view: &webkit2gtk::WebView, start: PendingMove, time: u32) -> bool {
    let Some(window) = gtk_window(view) else {
        drag_trace(|| "begin_move: no toplevel GtkWindow".to_owned());
        return false;
    };
    drag_trace(|| format!("begin_move: button={} root=({},{}) time={time}", start.button, start.root_x, start.root_y));
    // GTK 3's Wayland backend ignores this timestamp and sends the seat's
    // implicit-grab serial from the press that armed the move; the button is
    // still held, so the compositor accepts it. This is how GtkWindow starts
    // a move from its own title bar drag gesture. On X11 the motion event's
    // timestamp is the real one.
    unsafe {
        gtk_window_begin_move_drag(
            window,
            start.button as std::os::raw::c_int,
            start.root_x.round() as std::os::raw::c_int,
            start.root_y.round() as std::os::raw::c_int,
            time,
        );
    }
    true
}

#[cfg(target_os = "linux")]
fn drag_threshold() -> f64 {
    use webkit2gtk::glib::prelude::*;
    use webkit2gtk::glib::translate::FromGlibPtrNone;

    let settings = unsafe { gtk_settings_get_default() };
    if settings.is_null() {
        return DEFAULT_DRAG_THRESHOLD_PX;
    }
    let object: webkit2gtk::glib::Object = unsafe {
        webkit2gtk::glib::Object::from_glib_none(
            settings.cast::<webkit2gtk::glib::gobject_ffi::GObject>(),
        )
    };
    if object.find_property("gtk-dnd-drag-threshold").is_none() {
        return DEFAULT_DRAG_THRESHOLD_PX;
    }
    let value = object.property::<i32>("gtk-dnd-drag-threshold");
    if value > 0 {
        value as f64
    } else {
        DEFAULT_DRAG_THRESHOLD_PX
    }
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
fn event_state(event: *mut std::ffi::c_void) -> Option<u32> {
    let mut state = 0u32;
    let ok = unsafe { gdk_event_get_state(event, &mut state) };
    (ok != 0).then_some(state)
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
    fn gtk_settings_get_default() -> *mut std::ffi::c_void;
    fn gdk_event_get_state(event: *mut std::ffi::c_void, state: *mut u32) -> std::os::raw::c_int;
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
        button_mask, classify_title_press, linux_window_state_flags, parse_hit_message,
        passes_drag_threshold, pointer_in_css_space, CssRect, HitMessage, LiveTitleHitMap,
        MotionStep, PendingMove, PressKind, PressStep, TitleGesture, TitlePress, TitlePressAction,
        PRIMARY_BUTTON,
    };
    use tauri_plugin_window_state::StateFlags;

    const TOKEN: &str = "0123456789abcdef0123456789abcdef";

    fn press(kind: PressKind, x: f64, y: f64, live: Option<LiveTitleHitMap>) -> TitlePress {
        TitlePress {
            button: PRIMARY_BUTTON,
            kind,
            x,
            y,
            surface_width: 1280.0,
            zoom: 1.0,
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
            overlay: false,
        }
    }

    fn message(body: &str) -> String {
        format!(r#"{{"token":"{TOKEN}",{body}}}"#)
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
        assert!(!flags.contains(StateFlags::DECORATIONS));
        assert_ne!(flags.bits(), StateFlags::all().bits());
    }

    #[test]
    fn primary_press_on_the_drag_surface_arms_a_move() {
        let map = sample_map();
        assert_eq!(
            classify_title_press(&press(PressKind::Click, 40.0, 12.0, Some(map.clone()))),
            TitlePressAction::ArmMove
        );
        assert_eq!(
            classify_title_press(&press(PressKind::Click, 12.0, 2.0, Some(map))),
            TitlePressAction::ArmMove
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
    fn double_click_on_the_drag_surface_toggles_maximize() {
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
        assert_eq!(
            classify_title_press(&press(PressKind::Other, 80.0, 10.0, Some(map))),
            TitlePressAction::Ignore
        );
    }

    #[test]
    fn nothing_is_a_drag_surface_before_the_page_publishes() {
        assert_eq!(
            classify_title_press(&press(PressKind::Click, 20.0, 8.0, None)),
            TitlePressAction::Ignore
        );
        assert_eq!(
            classify_title_press(&press(PressKind::DoubleClick, 20.0, 8.0, None)),
            TitlePressAction::Ignore
        );
    }

    #[test]
    fn content_below_the_strip_is_not_a_drag_surface() {
        assert_eq!(
            classify_title_press(&press(PressKind::Click, 40.0, 80.0, Some(sample_map()))),
            TitlePressAction::Ignore
        );
    }

    #[test]
    fn an_overlay_over_the_strip_blocks_every_press() {
        let mut map = sample_map();
        map.overlay = true;
        assert_eq!(
            classify_title_press(&press(PressKind::Click, 40.0, 12.0, Some(map.clone()))),
            TitlePressAction::Ignore
        );
        assert_eq!(
            classify_title_press(&press(PressKind::DoubleClick, 40.0, 12.0, Some(map))),
            TitlePressAction::Ignore
        );
    }

    #[test]
    fn page_zoom_scales_the_pointer_and_a_stale_width_is_not_classified() {
        assert_eq!(
            pointer_in_css_space(100.0, 10.0, 1280.0, 1280.0, 1.0),
            Some((100.0, 10.0))
        );
        assert_eq!(
            pointer_in_css_space(200.0, 20.0, 2000.0, 1000.0, 2.0),
            Some((100.0, 10.0))
        );
        // Published before a resize: the press waits for the next map.
        assert_eq!(pointer_in_css_space(100.0, 10.0, 1400.0, 1280.0, 1.0), None);
        assert_eq!(pointer_in_css_space(100.0, 10.0, 1280.0, 0.0, 1.0), None);

        let mut stale = press(PressKind::Click, 40.0, 12.0, Some(sample_map()));
        stale.surface_width = 1600.0;
        assert_eq!(classify_title_press(&stale), TitlePressAction::Ignore);
    }

    #[test]
    fn parses_strip_geometry_with_the_token() {
        let HitMessage::Replace(Some(map)) = parse_hit_message(
            &message(
                r#""innerWidth":900,"drag":[{"x":0,"y":0,"width":700,"height":40}],"blocked":[{"x":700,"y":4,"width":180,"height":32}],"overlay":false"#,
            ),
            TOKEN,
        ) else {
            panic!("geometry payload");
        };
        assert_eq!(map.inner_width, 900.0);
        assert_eq!(map.drag.len(), 1);
        assert_eq!(map.blocked[0].x, 700.0);
        assert!(!map.overlay);

        // WebKit may hand the payload over as a JSON string.
        let quoted = serde_json::to_string(&message(
            r#""innerWidth":900,"drag":[{"x":0,"y":0,"width":700,"height":40}],"overlay":false"#,
        ))
        .unwrap();
        assert!(matches!(
            parse_hit_message(&quoted, TOKEN),
            HitMessage::Replace(Some(_))
        ));
    }

    #[test]
    fn messages_without_the_token_are_ignored() {
        let body =
            r#""innerWidth":900,"drag":[{"x":0,"y":0,"width":900,"height":900}],"overlay":false"#;
        assert_eq!(
            parse_hit_message(&format!("{{{body}}}"), TOKEN),
            HitMessage::Ignore
        );
        assert_eq!(
            parse_hit_message(&format!(r#"{{"token":"nope",{body}}}"#), TOKEN),
            HitMessage::Ignore
        );
        assert_eq!(parse_hit_message("not json", TOKEN), HitMessage::Ignore);
        assert_eq!(
            parse_hit_message(&message(r#""innerWidth":9"#), ""),
            HitMessage::Ignore
        );
    }

    #[test]
    fn an_empty_or_unshaped_map_clears_the_geometry() {
        // Strip unmounted (element fullscreen): clear, do not keep the old map.
        assert_eq!(
            parse_hit_message(
                &message(r#""innerWidth":900,"drag":[],"overlay":false"#),
                TOKEN
            ),
            HitMessage::Replace(None)
        );
        assert_eq!(
            parse_hit_message(
                &message(
                    r#""innerWidth":900,"drag":[{"x":0,"y":0,"width":0,"height":0}],"overlay":false"#
                ),
                TOKEN
            ),
            HitMessage::Replace(None)
        );
        // Taller than the strip can be, or below the top band.
        assert_eq!(
            parse_hit_message(
                &message(
                    r#""innerWidth":900,"drag":[{"x":0,"y":0,"width":900,"height":900}],"overlay":false"#
                ),
                TOKEN
            ),
            HitMessage::Replace(None)
        );
        assert_eq!(
            parse_hit_message(
                &message(
                    r#""innerWidth":900,"drag":[{"x":0,"y":200,"width":900,"height":40}],"overlay":false"#
                ),
                TOKEN
            ),
            HitMessage::Replace(None)
        );
        assert_eq!(
            parse_hit_message(
                &message(
                    r#""innerWidth":900,"drag":[{"x":0,"y":0,"width":null,"height":40}],"overlay":false"#
                ),
                TOKEN
            ),
            HitMessage::Replace(None)
        );
    }

    #[test]
    fn a_missing_overlay_flag_fails_closed() {
        let HitMessage::Replace(Some(map)) = parse_hit_message(
            &message(r#""innerWidth":900,"drag":[{"x":0,"y":0,"width":900,"height":40}]"#),
            TOKEN,
        ) else {
            panic!("geometry payload");
        };
        assert!(map.overlay);
    }

    #[test]
    fn drag_threshold_matches_gtk() {
        assert!(!passes_drag_threshold(10.0, 10.0, 18.0, 18.0, 8.0));
        assert!(passes_drag_threshold(10.0, 10.0, 18.5, 10.0, 8.0));
        assert!(passes_drag_threshold(10.0, 10.0, 10.0, 1.0, 8.0));
        assert!(!passes_drag_threshold(0.0, 0.0, 7.0, 0.0, f64::NAN));
        assert!(passes_drag_threshold(0.0, 0.0, 9.0, 0.0, f64::NAN));
    }

    #[test]
    fn button_masks_follow_gdk() {
        assert_eq!(button_mask(1), 1 << 8);
        assert_eq!(button_mask(3), 1 << 10);
        assert_eq!(button_mask(0), 0);
        assert_eq!(button_mask(9), 0);
    }

    #[test]
    fn a_press_moves_only_after_the_threshold_while_held() {
        let mut gesture = TitleGesture::default();
        assert_eq!(
            gesture.press(TitlePressAction::ArmMove, 1, 100.0, 50.0),
            PressStep::Consume
        );
        assert_eq!(gesture.motion(103.0, 51.0, true, 8.0), MotionStep::Wait);
        assert_eq!(
            gesture.motion(120.0, 50.0, true, 8.0),
            MotionStep::BeginMove(PendingMove {
                button: 1,
                root_x: 100.0,
                root_y: 50.0,
            })
        );
        // One move per press.
        assert_eq!(gesture.motion(140.0, 50.0, true, 8.0), MotionStep::Pass);
        assert!(!gesture.release());
    }

    #[test]
    fn a_click_without_travel_swallows_its_release_and_does_not_move() {
        let mut gesture = TitleGesture::default();
        gesture.press(TitlePressAction::ArmMove, 1, 100.0, 50.0);
        assert_eq!(gesture.motion(101.0, 50.0, true, 8.0), MotionStep::Wait);
        assert!(gesture.release());
        assert_eq!(gesture.motion(200.0, 50.0, true, 8.0), MotionStep::Pass);
    }

    #[test]
    fn double_click_cancels_the_armed_move_and_toggles_once() {
        let mut gesture = TitleGesture::default();
        // GDK: press, release, press, 2BUTTON_PRESS, release.
        gesture.press(TitlePressAction::ArmMove, 1, 100.0, 50.0);
        assert!(gesture.release());
        gesture.press(TitlePressAction::ArmMove, 1, 100.0, 50.0);
        assert_eq!(
            gesture.press(TitlePressAction::ToggleMaximize, 1, 100.0, 50.0),
            PressStep::ToggleMaximize
        );
        assert_eq!(gesture.motion(150.0, 50.0, true, 8.0), MotionStep::Pass);
        assert!(!gesture.release());
    }

    #[test]
    fn a_lost_release_disarms_on_the_next_motion() {
        let mut gesture = TitleGesture::default();
        gesture.press(TitlePressAction::ArmMove, 1, 100.0, 50.0);
        assert_eq!(gesture.motion(150.0, 50.0, false, 8.0), MotionStep::Pass);
        assert_eq!(gesture.motion(150.0, 50.0, true, 8.0), MotionStep::Pass);
    }

    #[test]
    fn an_ignored_press_disarms_and_passes_to_webkit() {
        let mut gesture = TitleGesture::default();
        gesture.press(TitlePressAction::ArmMove, 1, 100.0, 50.0);
        assert_eq!(
            gesture.press(TitlePressAction::Ignore, 3, 100.0, 50.0),
            PressStep::Pass
        );
        assert!(!gesture.release());
    }
}
