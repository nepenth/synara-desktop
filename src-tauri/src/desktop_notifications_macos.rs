//! Bundled macOS notifications use one UserNotifications client for permission,
//! submission receipts, presentation, and actions. macOS rejects legacy sends
//! after the same application connects through UserNotifications.

use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use block2::{DynBlock, RcBlock};
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{define_class, msg_send, AnyThread};
use objc2_foundation::{
    NSArray, NSDictionary, NSError, NSObject, NSObjectProtocol, NSSet, NSString,
};
use objc2_user_notifications::*;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Runtime};

use super::{
    handle_notification_response, is_critical_notification, is_time_sensitive_agent_approval,
    macos_delivery, response_for_action, sanitize_dismiss_key,
    sanitize_notification_action_context, sanitize_notification_actions,
    sanitize_notification_route, DesktopNotificationAction, DesktopNotificationActionContext,
    DesktopNotificationPayload, DesktopNotificationSoundPolicy, DismissKeyIndex,
    NotificationBinding, NotificationResponse,
};

const CONTEXT_KEY: &str = "synara-response";
/// A bound post waits this long for Notification Center's submission
/// callback; it no longer holds the session lock meanwhile.
const BOUND_SUBMISSION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
type ResponseHandler = Box<dyn Fn(&UNNotificationResponse) + Send + Sync>;
static RESPONSE_HANDLER: OnceLock<ResponseHandler> = OnceLock::new();
static DELEGATE: OnceLock<Retained<NotificationDelegate>> = OnceLock::new();
static NEXT_IDENTIFIER: AtomicU64 = AtomicU64::new(0);
static CATEGORIES: LazyLock<Mutex<BTreeMap<String, Vec<DesktopNotificationAction>>>> =
    LazyLock::new(|| Mutex::new(BTreeMap::new()));
static DISMISS_INDEX: LazyLock<Mutex<DismissKeyIndex<String>>> =
    LazyLock::new(|| Mutex::new(DismissKeyIndex::default()));

define_class!(
    // SAFETY: NSObject has no subclassing requirements. The class has no
    // mutable ivars; response handling uses a thread-safe immutable callback.
    #[unsafe(super = NSObject)]
    struct NotificationDelegate;

    unsafe impl NSObjectProtocol for NotificationDelegate {}

    unsafe impl UNUserNotificationCenterDelegate for NotificationDelegate {
        #[unsafe(method(userNotificationCenter:willPresentNotification:withCompletionHandler:))]
        fn will_present(
            &self,
            _center: &UNUserNotificationCenter,
            _notification: &UNNotification,
            completion: &DynBlock<dyn Fn(UNNotificationPresentationOptions)>,
        ) {
            completion.call((UNNotificationPresentationOptions::Banner
                | UNNotificationPresentationOptions::List
                | UNNotificationPresentationOptions::Sound,));
        }

        #[unsafe(method(userNotificationCenter:didReceiveNotificationResponse:withCompletionHandler:))]
        fn did_receive(
            &self,
            _center: &UNUserNotificationCenter,
            response: &UNNotificationResponse,
            completion: &DynBlock<dyn Fn()>,
        ) {
            if let Some(handler) = RESPONSE_HANDLER.get() {
                handler(response);
            }
            completion.call(());
        }
    }
);

// SAFETY: The delegate has no ivars or mutable NSObject state. Its methods
// only call the immutable Send + Sync handler or the supplied completion.
unsafe impl Send for NotificationDelegate {}
unsafe impl Sync for NotificationDelegate {}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ResponseContext {
    #[serde(default)]
    session_generation: Option<u64>,
    /// Account and process that posted the banner (absent before 2.3).
    #[serde(default)]
    account: Option<String>,
    #[serde(default)]
    process: Option<u64>,
    route: Option<String>,
    actions: Vec<DesktopNotificationAction>,
    action_context: Option<DesktopNotificationActionContext>,
    /// Kept so a relaunched process can rebuild its dismiss index.
    #[serde(default)]
    dismiss_keys: Vec<String>,
}

impl ResponseContext {
    fn binding(&self) -> NotificationBinding {
        NotificationBinding {
            session_generation: self.session_generation,
            account: self.account.clone(),
            process: self.process,
        }
    }
}

fn decode_context(json: &str) -> Option<ResponseContext> {
    let mut context: ResponseContext = serde_json::from_str(json).ok()?;
    context.route = context
        .route
        .map(sanitize_notification_route)
        .transpose()
        .ok()?;
    context.actions = sanitize_notification_actions(Some(context.actions)).unwrap_or_default();
    context.action_context = context
        .action_context
        .and_then(sanitize_notification_action_context);
    context.dismiss_keys = std::mem::take(&mut context.dismiss_keys)
        .iter()
        .filter_map(|key| sanitize_dismiss_key(key))
        .collect();
    Some(context)
}

fn retained_response_has_required_binding(context: &ResponseContext) -> bool {
    let critical = is_critical_notification(context.action_context.as_ref(), &context.actions);
    !critical || context.session_generation.is_some()
}

/// What a click or action on a delivered banner asks for; `None` when the
/// banner has nothing to do (no route, an unknown action).
fn response_for_context(context: &ResponseContext, action: &str) -> Option<NotificationResponse> {
    if action == "default" {
        return context
            .route
            .clone()
            .map(|route| NotificationResponse::Open { route });
    }
    context
        .actions
        .iter()
        .any(|item| item.id == action)
        .then(|| {
            response_for_action(
                action,
                context.route.as_deref(),
                context.action_context.clone(),
            )
        })
}

pub(super) fn initialize<R: Runtime>(app: &AppHandle<R>) {
    RESPONSE_HANDLER.get_or_init(|| {
        let app = app.clone();
        Box::new(move |response| {
            DISMISS_INDEX
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .unregister(response.notification().request().identifier().to_string());
            let user_info = response.notification().request().content().userInfo();
            let Some(value) = user_info.objectForKey(&NSString::from_str(CONTEXT_KEY)) else {
                return;
            };
            let Ok(value) = value.downcast::<NSString>() else {
                return;
            };
            let Some(context) = decode_context(&value.to_string()) else {
                return;
            };
            if !retained_response_has_required_binding(&context) {
                super::log_notification_error(
                    &app,
                    "an approval banner without a session binding was ignored",
                );
                return;
            }
            let action = response.actionIdentifier();
            let action = if &*action == unsafe { UNNotificationDefaultActionIdentifier } {
                "default".to_owned()
            } else {
                action.to_string()
            };
            if let Some(next) = response_for_context(&context, &action) {
                let critical =
                    is_critical_notification(context.action_context.as_ref(), &context.actions);
                handle_notification_response(&app, context.binding(), critical, next);
            }
        })
    });
    let delegate = DELEGATE.get_or_init(|| {
        // SAFETY: NSObject's initializer is valid for this ivar-free subclass.
        unsafe { msg_send![NotificationDelegate::alloc(), init] }
    });
    // The center stores a weak reference; DELEGATE retains it for app lifetime.
    UNUserNotificationCenter::currentNotificationCenter()
        .setDelegate(Some(ProtocolObject::from_ref(&**delegate)));
}

/// Only a granted permission posts; `default` (not asked yet) does not.
fn posts_with_permission(permission: &str) -> bool {
    permission == "granted"
}

pub(super) async fn permission() -> Result<String, String> {
    let status = macos_delivery::authorization_status().await?;
    Ok(permission_name(status))
}

fn permission_name(status: UNAuthorizationStatus) -> String {
    if status == UNAuthorizationStatus::NotDetermined {
        "default"
    } else if status == UNAuthorizationStatus::Authorized
        || status == UNAuthorizationStatus::Provisional
        || status == UNAuthorizationStatus::Ephemeral
    {
        "granted"
    } else {
        "denied"
    }
    .to_owned()
}

pub(super) async fn request_permission() -> Result<String, String> {
    let options = authorization_options(critical_alerts_supported());
    let (tx, rx) = tokio::sync::oneshot::channel();
    {
        let tx = Mutex::new(Some(tx));
        let completion =
            RcBlock::new(move |_granted: objc2::runtime::Bool, error: *mut NSError| {
                let result = if error.is_null() {
                    Ok(())
                } else {
                    Err("macOS notification permission request failed".to_owned())
                };
                if let Some(tx) = tx.lock().unwrap_or_else(|p| p.into_inner()).take() {
                    let _ = tx.send(result);
                }
            });
        UNUserNotificationCenter::currentNotificationCenter()
            .requestAuthorizationWithOptions_completionHandler(options, &completion);
    }
    // A user may need time to answer the OS prompt; this is not a delivery retry.
    tokio::time::timeout(std::time::Duration::from_secs(60), rx)
        .await
        .map_err(|_| "macOS notification permission request timed out".to_owned())?
        .map_err(|_| "macOS notification permission request closed".to_owned())??;
    // Denying the additional Critical capability must not mask ordinary
    // notification authorization that is already enabled.
    permission().await
}

fn authorization_options(critical_alerts_supported: bool) -> UNAuthorizationOptions {
    let options = UNAuthorizationOptions::Alert
        | UNAuthorizationOptions::Sound
        | UNAuthorizationOptions::Badge;
    if critical_alerts_supported {
        options | UNAuthorizationOptions::CriticalAlert
    } else {
        options
    }
}

fn critical_alerts_supported() -> bool {
    static SUPPORTED: OnceLock<bool> = OnceLock::new();
    *SUPPORTED.get_or_init(|| {
        use std::ffi::c_void;
        // Read the current executable's signed entitlement through the public
        // macOS Security API. OS settings alone do not establish capability.
        #[link(name = "Security", kind = "framework")]
        extern "C" {
            fn SecTaskCreateFromSelf(allocator: *const c_void) -> *const c_void;
            fn SecTaskCopyValueForEntitlement(
                task: *const c_void,
                entitlement: *const c_void,
                error: *mut *const c_void,
            ) -> *const c_void;
        }
        #[link(name = "CoreFoundation", kind = "framework")]
        extern "C" {
            fn CFRelease(value: *const c_void);
            fn CFGetTypeID(value: *const c_void) -> usize;
            fn CFBooleanGetTypeID() -> usize;
            fn CFBooleanGetValue(value: *const c_void) -> bool;
        }
        let key = NSString::from_str("com.apple.developer.usernotifications.critical-alerts");
        // SAFETY: SecTask returns owned CF objects or null. NSString is
        // toll-free bridged to CFString. Values are type-checked as CFBoolean
        // before access, and both owned objects are released exactly once.
        unsafe {
            let task = SecTaskCreateFromSelf(std::ptr::null());
            if task.is_null() {
                return false;
            }
            let value = SecTaskCopyValueForEntitlement(
                task,
                (&*key as *const NSString).cast(),
                std::ptr::null_mut(),
            );
            CFRelease(task);
            if value.is_null() {
                return false;
            }
            let supported = CFGetTypeID(value) == CFBooleanGetTypeID() && CFBooleanGetValue(value);
            CFRelease(value);
            supported
        }
    })
}

fn approval_interruption_level(
    critical_setting: UNNotificationSetting,
    silent: bool,
) -> UNNotificationInterruptionLevel {
    if critical_setting == UNNotificationSetting::Enabled && !silent {
        UNNotificationInterruptionLevel::Critical
    } else {
        UNNotificationInterruptionLevel::TimeSensitive
    }
}

/// Registers the action set's category and, when it is new, waits until the
/// notification center reports it. `setNotificationCategories` applies
/// asynchronously; a request posted before that completes is delivered with
/// an unknown category and therefore without any action buttons.
async fn ensure_category(actions: &[DesktopNotificationAction]) -> String {
    let (identifier, newly_registered) = register_category(actions);
    if newly_registered {
        for _ in 0..20 {
            if registered_category_identifiers()
                .await
                .contains(&identifier)
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    }
    identifier
}

async fn registered_category_identifiers() -> Vec<String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    {
        let tx = Mutex::new(Some(tx));
        let completion = RcBlock::new(
            move |categories: std::ptr::NonNull<NSSet<UNNotificationCategory>>| {
                // The set is valid for the duration of this callback only.
                let categories = unsafe { categories.as_ref() };
                let identifiers = categories
                    .iter()
                    .map(|category| category.identifier().to_string())
                    .collect::<Vec<_>>();
                if let Some(tx) = tx.lock().unwrap_or_else(|p| p.into_inner()).take() {
                    let _ = tx.send(identifiers);
                }
            },
        );
        UNUserNotificationCenter::currentNotificationCenter()
            .getNotificationCategoriesWithCompletionHandler(&completion);
    }
    tokio::time::timeout(std::time::Duration::from_secs(2), rx)
        .await
        .ok()
        .and_then(Result::ok)
        .unwrap_or_default()
}

fn register_category(actions: &[DesktopNotificationAction]) -> (String, bool) {
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    for action in actions {
        action.id.hash(&mut hash);
        action.label.hash(&mut hash);
    }
    let identifier = format!("synara-actions-{:x}", hash.finish());
    let mut categories = CATEGORIES.lock().unwrap_or_else(|p| p.into_inner());
    if categories.contains_key(&identifier) {
        return (identifier, false);
    }
    categories.insert(identifier.clone(), actions.to_vec());
    if categories.len() > 64 {
        if let Some(oldest) = categories.keys().find(|key| **key != identifier).cloned() {
            categories.remove(&oldest);
        }
    }
    let native_categories = categories
        .iter()
        .map(|(id, actions)| {
            let native_actions = actions
                .iter()
                .map(|action| {
                    UNNotificationAction::actionWithIdentifier_title_options(
                        &NSString::from_str(&action.id),
                        &NSString::from_str(&action.label),
                        notification_action_options(&action.id),
                    )
                })
                .collect::<Vec<_>>();
            UNNotificationCategory::categoryWithIdentifier_actions_intentIdentifiers_options(
                &NSString::from_str(id),
                &NSArray::from_retained_slice(&native_actions),
                &NSArray::<NSString>::new(),
                UNNotificationCategoryOptions::empty(),
            )
        })
        .collect::<Vec<_>>();
    UNUserNotificationCenter::currentNotificationCenter()
        .setNotificationCategories(&NSSet::from_retained_slice(&native_categories));
    (identifier, true)
}

/// Review opens the prompt, so it brings Synara forward even when hidden;
/// Approve once runs in the background, and Deny is marked destructive.
fn notification_action_options(action_id: &str) -> UNNotificationActionOptions {
    match action_id {
        "agent-approval.deny" => UNNotificationActionOptions::Destructive,
        "agent-approval.review" | "agent-approval.approve-always" => {
            UNNotificationActionOptions::Foreground
        }
        _ => UNNotificationActionOptions::empty(),
    }
}

fn remove_requests(identifiers: &[String]) {
    if identifiers.is_empty() {
        return;
    }
    let identifiers = identifiers
        .iter()
        .map(|id| NSString::from_str(id))
        .collect::<Vec<_>>();
    let identifiers = NSArray::from_retained_slice(&identifiers);
    let center = UNUserNotificationCenter::currentNotificationCenter();
    center.removePendingNotificationRequestsWithIdentifiers(&identifiers);
    center.removeDeliveredNotificationsWithIdentifiers(&identifiers);
}

pub(super) fn dismiss(keys: &[String]) {
    let ids = {
        let mut index = DISMISS_INDEX.lock().unwrap_or_else(|p| p.into_inner());
        let ids = index.ids_for_keys(keys);
        for id in &ids {
            index.unregister(id.clone());
        }
        ids
    };
    remove_requests(&ids);
}

/// Remove every request this app posted, delivered or pending, including an
/// earlier launch's, and forget the index.
pub(super) fn dismiss_all() {
    *DISMISS_INDEX.lock().unwrap_or_else(|p| p.into_inner()) = DismissKeyIndex::default();
    let center = UNUserNotificationCenter::currentNotificationCenter();
    center.removeAllPendingNotificationRequests();
    center.removeAllDeliveredNotifications();
}

/// Index the banners an earlier launch left in Notification Center by their
/// stored dismiss keys, so reading a room or signing out still withdraws
/// them. Candidate keys are per process and are not carried over.
pub(super) fn rebuild_dismiss_index() {
    let completion = RcBlock::new(
        move |notifications: std::ptr::NonNull<NSArray<UNNotification>>| {
            // The array is valid for the duration of this callback only.
            let notifications = unsafe { notifications.as_ref() };
            let mut index = DISMISS_INDEX.lock().unwrap_or_else(|p| p.into_inner());
            for notification in notifications.iter() {
                let request = notification.request();
                let Some(keys) = delivered_dismiss_keys(&request.content().userInfo()) else {
                    continue;
                };
                // A banner this process posted meanwhile is already indexed.
                let identifier = request.identifier().to_string();
                if !index.keys_by_id.contains_key(&identifier) {
                    index.register(identifier, &keys);
                }
            }
        },
    );
    UNUserNotificationCenter::currentNotificationCenter()
        .getDeliveredNotificationsWithCompletionHandler(&completion);
}

fn delivered_dismiss_keys(user_info: &NSDictionary) -> Option<Vec<String>> {
    let value = user_info.objectForKey(&NSString::from_str(CONTEXT_KEY))?;
    let value = value.downcast::<NSString>().ok()?;
    let keys = carried_over_dismiss_keys(decode_context(&value.to_string())?);
    (!keys.is_empty()).then_some(keys)
}

fn carried_over_dismiss_keys(context: ResponseContext) -> Vec<String> {
    context
        .dismiss_keys
        .into_iter()
        .filter(|key| !key.starts_with("candidate:"))
        .collect()
}

/// Whether the banner asks Notification Center to play the default sound.
/// Messages sound unless the renderer asked for silence (sounds off, or the
/// push rule has no sound); macOS Focus then decides whether it plays.
fn message_sound_enabled(sound: Option<DesktopNotificationSoundPolicy>) -> bool {
    sound != Some(DesktopNotificationSoundPolicy::Silent)
}

pub(super) async fn show<R: Runtime>(
    app: &AppHandle<R>,
    notification: &DesktopNotificationPayload,
    binding: &NotificationBinding,
) -> Result<bool, String> {
    let title = notification.title.as_str();
    let body = notification.body.as_deref();
    let route = notification.route.as_deref();
    let actions = notification.actions.as_deref().unwrap_or(&[]);
    let action_context = notification.action_context.as_ref();
    let dismiss_keys = notification.dismiss_keys.as_deref().unwrap_or(&[]);
    // Never ask from here: the OS prompt would appear mid-use when the first
    // message arrives. Until permission is granted (Settings, or the
    // renderer's one-time request), the post is reported undelivered.
    if !posts_with_permission(&permission().await?) {
        return Ok(false);
    }
    initialize(app);
    let (_, critical_setting) = macos_delivery::notification_settings().await?;
    let critical_setting = if critical_alerts_supported() {
        critical_setting
    } else {
        UNNotificationSetting::NotSupported
    };
    let category = if actions.is_empty() {
        None
    } else {
        Some(ensure_category(actions).await)
    };
    let (rx, identifier) = {
        let content = UNMutableNotificationContent::new();
        content.setTitle(&NSString::from_str(title));
        if let Some(body) = body {
            content.setBody(&NSString::from_str(body));
        }
        if is_time_sensitive_agent_approval(action_context) {
            let silent = notification.sound == Some(DesktopNotificationSoundPolicy::Silent);
            let level = approval_interruption_level(critical_setting, silent);
            content.setSubtitle(&NSString::from_str(
                if level == UNNotificationInterruptionLevel::Critical {
                    "Critical approval · expires in 5 minutes"
                } else {
                    "Time-sensitive · expires in 5 minutes"
                },
            ));
            if !silent {
                let sound = if level == UNNotificationInterruptionLevel::Critical {
                    UNNotificationSound::defaultCriticalSound()
                } else {
                    UNNotificationSound::defaultSound()
                };
                content.setSound(Some(&sound));
            }
            content.setInterruptionLevel(level);
        } else if message_sound_enabled(notification.sound) {
            content.setSound(Some(&UNNotificationSound::defaultSound()));
        }
        if let Some(category) = category.as_deref() {
            content.setCategoryIdentifier(&NSString::from_str(category));
        }
        let context = serde_json::to_string(&ResponseContext {
            session_generation: notification.session_generation,
            account: binding.account.clone(),
            process: binding.process,
            route: route.map(str::to_owned),
            actions: actions.to_vec(),
            action_context: action_context.cloned(),
            dismiss_keys: dismiss_keys.to_vec(),
        })
        .map_err(|_| "macOS notification response encoding failed".to_owned())?;
        let key = NSString::from_str(CONTEXT_KEY);
        let value = NSString::from_str(&context);
        let user_info = NSDictionary::<NSString, NSString>::from_slices(&[&*key], &[&*value]);
        // SAFETY: Erasing the dictionary's generics is valid: both keys and values
        // are NSString property-list objects, as required by UNNotificationContent.
        let user_info: Retained<NSDictionary> = unsafe { Retained::cast_unchecked(user_info) };
        unsafe { content.setUserInfo(&user_info) };
        let identifier = NSString::from_str(&format!(
            "synara-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            NEXT_IDENTIFIER.fetch_add(1, Ordering::Relaxed)
        ));
        let identifier_text = identifier.to_string();
        let evicted = DISMISS_INDEX
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .register(identifier_text.clone(), dismiss_keys);
        remove_requests(&evicted);
        let request = UNNotificationRequest::requestWithIdentifier_content_trigger(
            &identifier,
            &content,
            None,
        );
        let (tx, rx) = tokio::sync::oneshot::channel();
        {
            let tx = Mutex::new(Some(tx));
            let completion = RcBlock::new(move |error: *mut NSError| {
                if let Some(tx) = tx.lock().unwrap_or_else(|p| p.into_inner()).take() {
                    let _ = tx.send(error.is_null());
                }
            });
            UNUserNotificationCenter::currentNotificationCenter()
                .addNotificationRequest_withCompletionHandler(&request, Some(&completion));
        }
        (rx, identifier_text)
    };
    // The submission callback is the OS acceptance receipt. It is independent
    // of banner visibility and of later click/action callbacks. Never resend.
    // Bound posts wait longer, without the session lock: the caller checks
    // the generation again afterwards and withdraws the banner if it ended.
    // Unbound system notices keep the existing receipt deadline. This
    // governs acceptance, not display.
    let deadline = if notification.session_generation.is_some() {
        BOUND_SUBMISSION_TIMEOUT
    } else {
        macos_delivery::RECEIPT_TIMEOUT
    };
    let accepted = tokio::time::timeout(deadline, rx)
        .await
        .map_err(|_| "macOS notification submission timed out".to_owned())
        .and_then(|result| result.map_err(|_| "macOS notification submission closed".to_owned()));
    let remove = settle_submission_receipt(
        &mut DISMISS_INDEX.lock().unwrap_or_else(|p| p.into_inner()),
        &identifier,
        dismiss_keys.is_empty(),
        matches!(accepted, Ok(true)),
    );
    if remove {
        remove_requests(&[identifier]);
        return accepted.map(|_| false);
    }
    accepted
}

// A dismissal may race OS acceptance. If the request was removed while its
// callback was pending, remove it again after acceptance so it cannot survive
// as a delivered banner. An exact candidate key never removes its successor.
fn settle_submission_receipt(
    index: &mut DismissKeyIndex<String>,
    identifier: &str,
    unkeyed: bool,
    accepted: bool,
) -> bool {
    if !accepted || (!unkeyed && !index.keys_by_id.contains_key(identifier)) {
        index.unregister(identifier.to_owned());
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn critical_permission_requires_os_capability() {
        use super::*;
        assert!(!authorization_options(false).contains(UNAuthorizationOptions::CriticalAlert));
        assert!(authorization_options(true).contains(UNAuthorizationOptions::CriticalAlert));
    }

    #[test]
    fn critical_approval_requires_authorization_and_audible_delivery() {
        use super::*;
        for setting in [
            UNNotificationSetting::NotSupported,
            UNNotificationSetting::Disabled,
        ] {
            assert_eq!(
                approval_interruption_level(setting, false),
                UNNotificationInterruptionLevel::TimeSensitive
            );
        }
        assert_eq!(
            approval_interruption_level(UNNotificationSetting::Enabled, false),
            UNNotificationInterruptionLevel::Critical
        );
        assert_eq!(
            approval_interruption_level(UNNotificationSetting::Enabled, true),
            UNNotificationInterruptionLevel::TimeSensitive
        );
    }

    use super::*;

    #[test]
    fn late_os_receipt_repeats_exact_removal_without_dismissing_successor() {
        let mut index = DismissKeyIndex::<String>::default();
        index.register(
            "old-request".to_owned(),
            &["candidate:notif-7-1".to_owned(), "event:$same".to_owned()],
        );
        index.register(
            "new-request".to_owned(),
            &["candidate:notif-8-1".to_owned(), "event:$same".to_owned()],
        );
        let cancelled = index.ids_for_keys(&["candidate:notif-7-1".to_owned()]);
        assert_eq!(cancelled, vec!["old-request".to_owned()]);
        for id in cancelled {
            index.unregister(id);
        }
        assert!(settle_submission_receipt(
            &mut index,
            "old-request",
            false,
            true
        ));
        assert!(!settle_submission_receipt(
            &mut index,
            "new-request",
            false,
            true
        ));
        assert_eq!(
            index.ids_for_keys(&["event:$same".to_owned()]),
            vec!["new-request".to_owned()]
        );
        assert!(settle_submission_receipt(
            &mut index,
            "new-request",
            false,
            false
        ));
        assert!(index.ids_for_keys(&["event:$same".to_owned()]).is_empty());
    }

    #[test]
    fn persisted_os_response_preserves_original_session_generation() {
        let bound = decode_context(r#"{"sessionGeneration":7,"route":"/inbox/later/","actions":[{"id":"agent-approval.approve-once","label":"Approve once"}],"actionContext":{"kind":"agent-approval","roomId":"!shared:example.org","eventId":"$same"}}"#).unwrap();
        assert_eq!(bound.session_generation, Some(7));
        assert!(retained_response_has_required_binding(&bound));
        let round_trip = decode_context(&serde_json::to_string(&bound).unwrap()).unwrap();
        assert_eq!(round_trip.session_generation, Some(7));
        let old = decode_context(r#"{"route":null,"actions":[],"actionContext":null}"#).unwrap();
        assert!(old.session_generation.is_none());
        assert!(retained_response_has_required_binding(&old));
        let mut legacy_critical = bound;
        legacy_critical.session_generation = None;
        assert!(!retained_response_has_required_binding(&legacy_critical));
    }

    #[test]
    fn permission_not_asked_yet_reads_default_and_never_posts() {
        assert_eq!(
            permission_name(UNAuthorizationStatus::NotDetermined),
            "default"
        );
        assert_eq!(
            permission_name(UNAuthorizationStatus::Authorized),
            "granted"
        );
        assert_eq!(
            permission_name(UNAuthorizationStatus::Provisional),
            "granted"
        );
        assert_eq!(permission_name(UNAuthorizationStatus::Denied), "denied");
        assert!(posts_with_permission("granted"));
        assert!(!posts_with_permission("default"));
        assert!(!posts_with_permission("denied"));
        // The post path only reads permission; asking is left to Settings
        // and the renderer's one-time request.
        let source = include_str!("desktop_notifications_macos.rs");
        let show = source
            .split("pub(super) async fn show<")
            .nth(1)
            .and_then(|rest| rest.split("\nfn settle_submission_receipt").next())
            .expect("show body");
        assert!(!show.contains("request_permission"));
        assert!(!show.contains("rx.await"));
    }

    #[test]
    fn messages_carry_the_default_sound_unless_silent() {
        assert!(message_sound_enabled(Some(
            DesktopNotificationSoundPolicy::Default
        )));
        assert!(message_sound_enabled(None));
        assert!(!message_sound_enabled(Some(
            DesktopNotificationSoundPolicy::Silent
        )));
        let source = include_str!("desktop_notifications_macos.rs");
        assert!(source.contains(
            "} else if message_sound_enabled(notification.sound) {\n            content.setSound(Some(&UNNotificationSound::defaultSound()));"
        ));
    }

    #[test]
    fn review_brings_synara_forward_and_approve_stays_in_the_background() {
        assert_eq!(
            notification_action_options("agent-approval.review"),
            UNNotificationActionOptions::Foreground
        );
        assert_eq!(
            notification_action_options("agent-approval.deny"),
            UNNotificationActionOptions::Destructive
        );
        assert_eq!(
            notification_action_options("agent-approval.approve-once"),
            UNNotificationActionOptions::empty()
        );
        let context = decode_context(r#"{"sessionGeneration":3,"route":"/direct/!a:x/$e","actions":[{"id":"agent-approval.review","label":"Review"},{"id":"agent-approval.deny","label":"Deny"}],"actionContext":{"kind":"agent-approval","roomId":"!a:x","eventId":"$e"}}"#).unwrap();
        assert_eq!(
            response_for_context(&context, "agent-approval.review"),
            Some(NotificationResponse::Open {
                route: "/direct/!a:x/$e".to_owned()
            })
        );
        assert!(matches!(
            response_for_context(&context, "agent-approval.deny"),
            Some(NotificationResponse::Action { .. })
        ));
        assert_eq!(response_for_context(&context, "agent-approval.other"), None);
        assert!(matches!(
            response_for_context(&context, "default"),
            Some(NotificationResponse::Open { .. })
        ));
    }

    #[test]
    fn a_relaunch_reindexes_stored_dismiss_keys_but_not_candidates() {
        let context = decode_context(r#"{"sessionGeneration":3,"account":"@a:x","process":9,"route":null,"actions":[],"actionContext":null,"dismissKeys":["room:!a:x","candidate:notif-3-1","bad key","invite:!b:x"]}"#).unwrap();
        assert_eq!(
            context.binding(),
            NotificationBinding {
                session_generation: Some(3),
                account: Some("@a:x".to_owned()),
                process: Some(9),
            }
        );
        assert_eq!(
            carried_over_dismiss_keys(context),
            vec!["room:!a:x".to_owned(), "invite:!b:x".to_owned()]
        );
    }

    #[test]
    fn persisted_response_rejects_external_routes() {
        assert!(decode_context(
            r#"{"route":"https://example.org","actions":[],"actionContext":null}"#
        )
        .is_none());
    }

    #[test]
    fn persisted_response_preserves_only_valid_actions() {
        let context = decode_context(r#"{"route":"/inbox/later/","actions":[{"id":"agent-approval.deny","label":"Deny"},{"id":"bad id","label":"Bad"}],"actionContext":null}"#).unwrap();
        assert_eq!(context.actions.len(), 1);
        assert_eq!(context.actions[0].id, "agent-approval.deny");
    }
}
