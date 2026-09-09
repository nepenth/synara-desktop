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
    navigate_main_window, sanitize_notification_action_context, sanitize_notification_actions,
    sanitize_notification_route, DesktopNotificationAction, DesktopNotificationActionContext,
};

const CONTEXT_KEY: &str = "synara-response";
type ResponseHandler = Box<dyn Fn(&UNNotificationResponse) + Send + Sync>;
static RESPONSE_HANDLER: OnceLock<ResponseHandler> = OnceLock::new();
static DELEGATE: OnceLock<Retained<NotificationDelegate>> = OnceLock::new();
static NEXT_IDENTIFIER: AtomicU64 = AtomicU64::new(0);
static CATEGORIES: LazyLock<Mutex<BTreeMap<String, Vec<DesktopNotificationAction>>>> =
    LazyLock::new(|| Mutex::new(BTreeMap::new()));

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

pub(super) fn initialize<R: Runtime>(app: &AppHandle<R>) {
    RESPONSE_HANDLER.get_or_init(|| {
        let app = app.clone();
        Box::new(move |response| {
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
            let action = response.actionIdentifier();
            if &*action == unsafe { UNNotificationDefaultActionIdentifier } {
                if let Some(route) = context.route {
                    let _ = navigate_main_window(&app, &route);
                }
            } else if context
                .actions
                .iter()
                .any(|item| item.id == action.to_string())
            {
                let _ = emit_notification_action(&app, &action.to_string(), context.action_context);
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

pub(super) async fn show<R: Runtime>(
    app: &AppHandle<R>,
    title: &str,
    body: Option<&str>,
    route: Option<&str>,
    actions: &[DesktopNotificationAction],
    action_context: Option<&DesktopNotificationActionContext>,
) -> Result<bool, String> {
    let permission = permission().await?;
    if permission == "denied"
        || (permission == "default" && request_permission().await? != "granted")
    {
        return Ok(false);
    }
    initialize(app);
    let rx = {
        let content = UNMutableNotificationContent::new();
        content.setTitle(&NSString::from_str(title));
        if let Some(body) = body {
            content.setBody(&NSString::from_str(body));
        }
        if is_time_sensitive_agent_approval(action_context) {
            content.setSubtitle(&NSString::from_str("Time-sensitive · expires in 5 minutes"));
            content.setSound(Some(&UNNotificationSound::defaultSound()));
            content.setInterruptionLevel(UNNotificationInterruptionLevel::TimeSensitive);
        }
        if !actions.is_empty() {
            content.setCategoryIdentifier(&register_category(actions));
        }
        let context = serde_json::to_string(&ResponseContext {
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
        rx
    };
    // The submission callback is the OS acceptance receipt. It is independent
    // of banner visibility and of later click/action callbacks. Never resend.
    tokio::time::timeout(macos_delivery::RECEIPT_TIMEOUT, rx)
        .await
        .map_err(|_| "macOS notification submission timed out".to_owned())?
        .map_err(|_| "macOS notification submission closed".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

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
