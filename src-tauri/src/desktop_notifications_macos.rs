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
    emit_notification_action, is_time_sensitive_agent_approval, macos_delivery,
    navigate_notification_route, sanitize_notification_action_context,
    sanitize_notification_actions, sanitize_notification_route, DesktopNotificationAction,
    DesktopNotificationActionContext, DesktopNotificationPayload, DesktopNotificationSoundPolicy,
    DismissKeyIndex,
};

const CONTEXT_KEY: &str = "synara-response";
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
    route: Option<String>,
    actions: Vec<DesktopNotificationAction>,
    action_context: Option<DesktopNotificationActionContext>,
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
    Some(context)
}

fn retained_response_has_required_binding(context: &ResponseContext) -> bool {
    let critical = is_time_sensitive_agent_approval(context.action_context.as_ref())
        || context
            .actions
            .iter()
            .any(|action| action.id.starts_with("agent-approval."));
    !critical || context.session_generation.is_some()
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
                return;
            }
            let action = response.actionIdentifier();
            if &*action == unsafe { UNNotificationDefaultActionIdentifier } {
                if let Some(route) = context.route {
                    let _ = navigate_notification_route(
                        &app,
                        &route,
                        context.session_generation,
                        false,
                    );
                }
            } else if context
                .actions
                .iter()
                .any(|item| item.id == action.to_string())
            {
                let _ = emit_notification_action(
                    &app,
                    &action.to_string(),
                    context.action_context,
                    context.session_generation,
                );
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

pub(super) async fn permission() -> Result<String, String> {
    let status = macos_delivery::authorization_status().await?;
    Ok(if status == UNAuthorizationStatus::NotDetermined {
        "default"
    } else if status == UNAuthorizationStatus::Authorized
        || status == UNAuthorizationStatus::Provisional
        || status == UNAuthorizationStatus::Ephemeral
    {
        "granted"
    } else {
        "denied"
    }
    .to_owned())
}

pub(super) async fn request_permission() -> Result<String, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    {
        let tx = Mutex::new(Some(tx));
        let completion = RcBlock::new(move |granted: objc2::runtime::Bool, error: *mut NSError| {
            let result = if error.is_null() {
                Ok(if granted.as_bool() {
                    "granted"
                } else {
                    "denied"
                }
                .to_owned())
            } else {
                Err("macOS notification permission request failed".to_owned())
            };
            if let Some(tx) = tx.lock().unwrap_or_else(|p| p.into_inner()).take() {
                let _ = tx.send(result);
            }
        });
        UNUserNotificationCenter::currentNotificationCenter()
            .requestAuthorizationWithOptions_completionHandler(
                UNAuthorizationOptions::Alert
                    | UNAuthorizationOptions::Sound
                    | UNAuthorizationOptions::Badge,
                &completion,
            );
    }
    // A user may need time to answer the OS prompt; this is not a delivery retry.
    tokio::time::timeout(std::time::Duration::from_secs(60), rx)
        .await
        .map_err(|_| "macOS notification permission request timed out".to_owned())?
        .map_err(|_| "macOS notification permission request closed".to_owned())?
}

fn register_category(actions: &[DesktopNotificationAction]) -> Retained<NSString> {
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    for action in actions {
        action.id.hash(&mut hash);
        action.label.hash(&mut hash);
    }
    let identifier = format!("synara-actions-{:x}", hash.finish());
    let mut categories = CATEGORIES.lock().unwrap_or_else(|p| p.into_inner());
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
                        UNNotificationActionOptions::empty(),
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
    NSString::from_str(&identifier)
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

pub(super) async fn show<R: Runtime>(
    app: &AppHandle<R>,
    notification: &DesktopNotificationPayload,
) -> Result<bool, String> {
    let title = notification.title.as_str();
    let body = notification.body.as_deref();
    let route = notification.route.as_deref();
    let actions = notification.actions.as_deref().unwrap_or(&[]);
    let action_context = notification.action_context.as_ref();
    let dismiss_keys = notification.dismiss_keys.as_deref().unwrap_or(&[]);
    let permission = permission().await?;
    if permission == "denied"
        || (permission == "default" && request_permission().await? != "granted")
    {
        return Ok(false);
    }
    initialize(app);
    let (rx, identifier) = {
        let content = UNMutableNotificationContent::new();
        content.setTitle(&NSString::from_str(title));
        if let Some(body) = body {
            content.setBody(&NSString::from_str(body));
        }
        if is_time_sensitive_agent_approval(action_context) {
            content.setSubtitle(&NSString::from_str("Time-sensitive · expires in 5 minutes"));
            if notification.sound != Some(DesktopNotificationSoundPolicy::Silent) {
                content.setSound(Some(&UNNotificationSound::defaultSound()));
            }
            content.setInterruptionLevel(UNNotificationInterruptionLevel::TimeSensitive);
        }
        if !actions.is_empty() {
            content.setCategoryIdentifier(&register_category(actions));
        }
        let context = serde_json::to_string(&ResponseContext {
            session_generation: notification.session_generation,
            route: route.map(str::to_owned),
            actions: actions.to_vec(),
            action_context: action_context.cloned(),
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
    // Bound posts retain the auth transition gate until the actual OS
    // callback completes. A renderer cancellation only drops its waiter;
    // the owned acceptance task keeps running. Unbound system notices keep
    // the existing receipt deadline. This governs acceptance, not display.
    let accepted = if notification.session_generation.is_some() {
        rx.await
            .map_err(|_| "macOS notification submission closed".to_owned())
    } else {
        tokio::time::timeout(macos_delivery::RECEIPT_TIMEOUT, rx)
            .await
            .map_err(|_| "macOS notification submission timed out".to_owned())
            .and_then(|result| {
                result.map_err(|_| "macOS notification submission closed".to_owned())
            })
    };
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
