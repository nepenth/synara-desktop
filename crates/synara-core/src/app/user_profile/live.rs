//! Live own-profile, ignore-list, 3PID, and avatar-upload reads and writes.

use std::sync::Mutex;

use matrix_sdk::ruma::api::client::uiaa::{
    AuthData, AuthType, MatrixUserIdentifier, Password, UserIdentifier,
};
use matrix_sdk::ruma::thirdparty::Medium;
use matrix_sdk::ruma::{ClientSecret, OwnedClientSecret, OwnedMxcUri, OwnedSessionId};
use matrix_sdk::Client;
use mime::Mime;

use super::{
    MatrixIgnoredUsersSnapshot, MatrixIgnoredUsersWriteResult, MatrixOwnProfile,
    MatrixProfileWriteResult, MatrixThreepidAddResult, MatrixThreepidEmail,
    MatrixThreepidEmailTokenResult, MatrixThreepidSnapshot, MatrixThreepidWriteResult,
    MatrixUploadAvatarResult, MatrixUserDirectoryHit, MatrixUserDirectorySearchResult,
};

pub const MAX_AVATAR_UPLOAD_BYTES: usize = 8 * 1024 * 1024;

const MAX_OWN_DISPLAY_NAME_CHARS: usize = 255;

pub const MAX_USER_DIRECTORY_TERM_CHARS: usize = 256;
pub const MAX_USER_DIRECTORY_LIMIT: u64 = 50;
pub const DEFAULT_USER_DIRECTORY_LIMIT: u64 = 10;

pub fn parse_own_display_name(display_name: &str) -> Result<Option<String>, &'static str> {
    let trimmed = display_name.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > MAX_OWN_DISPLAY_NAME_CHARS {
        return Err("v-send.r-avatar-display-name-too-long");
    }
    Ok(Some(trimmed.to_owned()))
}

pub fn parse_own_avatar_mxc(mxc: &str) -> Result<Option<OwnedMxcUri>, &'static str> {
    let trimmed = mxc.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if !trimmed.starts_with("mxc://") {
        return Err("v-send.r-avatar-invalid-mxc");
    }
    let owned = OwnedMxcUri::from(trimmed);
    if owned.as_str().matches('/').count() < 3 {
        return Err("v-send.r-avatar-invalid-mxc");
    }
    Ok(Some(owned))
}

pub async fn set_own_display_name(
    client: &Client,
    display_name: &str,
) -> Result<MatrixProfileWriteResult, &'static str> {
    let display_name = parse_own_display_name(display_name)?;
    client
        .account()
        .set_display_name(display_name.as_deref())
        .await
        .map_err(|_| "v-send.r-avatar-display-name-sdk-failed")?;
    Ok(MatrixProfileWriteResult { status: "ok" })
}

pub async fn set_own_avatar(
    client: &Client,
    mxc: &str,
) -> Result<MatrixProfileWriteResult, &'static str> {
    let mxc = parse_own_avatar_mxc(mxc)?;
    client
        .account()
        .set_avatar_url(mxc.as_deref())
        .await
        .map_err(|_| "v-send.r-avatar-set-sdk-failed")?;
    Ok(MatrixProfileWriteResult { status: "ok" })
}

pub async fn get_own_profile(client: &Client) -> Result<MatrixOwnProfile, &'static str> {
    let user_id = client
        .user_id()
        .ok_or("v-send.r-avatar-profile-no-session")?
        .to_string();
    let display_name = client
        .account()
        .get_display_name()
        .await
        .map_err(|_| "v-send.r-avatar-display-name-read-failed")?;
    if let Some(ref name) = display_name {
        parse_own_display_name(name)?;
    }
    let avatar = client
        .account()
        .get_avatar_url()
        .await
        .map_err(|_| "v-send.r-avatar-read-failed")?;
    let avatar_url = match avatar {
        Some(mxc) => {
            let serialized = mxc.to_string();
            parse_own_avatar_mxc(&serialized)?;
            Some(serialized)
        }
        None => None,
    };
    Ok(MatrixOwnProfile {
        user_id,
        display_name,
        avatar_url,
    })
}

pub fn parse_user_directory_term(term: &str) -> Result<String, &'static str> {
    let trimmed = term.trim();
    if trimmed.is_empty() {
        return Err("v-search.directory-empty-term");
    }
    if trimmed.chars().count() > MAX_USER_DIRECTORY_TERM_CHARS {
        return Err("v-search.directory-term-too-long");
    }
    if trimmed.contains("access_token") || trimmed.contains("refresh_token") {
        return Err("v-search.directory-invalid-term");
    }
    Ok(trimmed.to_owned())
}

pub fn parse_user_directory_limit(limit: Option<u64>) -> Result<u64, &'static str> {
    let limit = limit.unwrap_or(DEFAULT_USER_DIRECTORY_LIMIT);
    if !(1..=MAX_USER_DIRECTORY_LIMIT).contains(&limit) {
        return Err("v-search.directory-invalid-limit");
    }
    Ok(limit)
}

fn map_user_directory_hit(
    user_id: String,
    display_name: Option<String>,
    avatar_url: Option<String>,
) -> Option<MatrixUserDirectoryHit> {
    if matrix_sdk::ruma::UserId::parse(user_id.as_str()).is_err() {
        return None;
    }
    let display_name = display_name.and_then(|name| parse_own_display_name(&name).ok().flatten());
    let avatar_url = avatar_url.and_then(|mxc| {
        parse_own_avatar_mxc(&mxc)
            .ok()
            .flatten()
            .map(|owned| owned.to_string())
    });
    Some(MatrixUserDirectoryHit {
        user_id,
        display_name,
        avatar_url,
    })
}

pub async fn search_user_directory(
    client: &Client,
    term: &str,
    limit: Option<u64>,
) -> Result<MatrixUserDirectorySearchResult, &'static str> {
    let _ = client.user_id().ok_or("v-search.directory-no-session")?;
    let term = parse_user_directory_term(term)?;
    let limit = parse_user_directory_limit(limit)?;
    let response = client
        .search_users(&term, limit)
        .await
        .map_err(|_| "v-search.directory-sdk-failed")?;
    let mut results = Vec::new();
    for user in response.results {
        let Some(hit) = map_user_directory_hit(
            user.user_id.to_string(),
            user.display_name,
            user.avatar_url.map(|mxc| mxc.to_string()),
        ) else {
            continue;
        };
        results.push(hit);
    }
    if results.len() > limit as usize {
        results.truncate(limit as usize);
    }
    Ok(MatrixUserDirectorySearchResult {
        limited: response.limited,
        results,
    })
}

fn parse_ignored_user_id(user_id: &str) -> Result<matrix_sdk::ruma::OwnedUserId, &'static str> {
    let trimmed = user_id.trim();
    if trimmed.is_empty() || !trimmed.starts_with('@') {
        return Err("v-profile.ignore-invalid-user");
    }
    matrix_sdk::ruma::UserId::parse(trimmed).map_err(|_| "v-profile.ignore-invalid-user")
}

pub async fn snapshot_ignored_users(
    client: &Client,
) -> Result<MatrixIgnoredUsersSnapshot, &'static str> {
    let _ = client.user_id().ok_or("v-profile.ignore-no-session")?;
    let content = client
        .account()
        .account_data::<matrix_sdk::ruma::events::ignored_user_list::IgnoredUserListEventContent>()
        .await
        .map_err(|_| "v-profile.ignore-snapshot-failed")?
        .map(|raw| raw.deserialize())
        .transpose()
        .map_err(|_| "v-profile.ignore-snapshot-failed")?
        .unwrap_or_default();
    let mut user_ids: Vec<String> = content
        .ignored_users
        .into_keys()
        .map(|user_id| user_id.to_string())
        .collect();
    user_ids.sort();
    Ok(MatrixIgnoredUsersSnapshot { user_ids })
}

pub async fn ignore_user(
    client: &Client,
    user_id: &str,
) -> Result<MatrixIgnoredUsersWriteResult, &'static str> {
    let own = client.user_id().ok_or("v-profile.ignore-no-session")?;
    let parsed = parse_ignored_user_id(user_id)?;
    if parsed.as_str() == own.as_str() {
        return Err("v-profile.ignore-self");
    }
    client
        .account()
        .ignore_user(&parsed)
        .await
        .map_err(|_| "v-profile.ignore-sdk-failed")?;
    Ok(MatrixIgnoredUsersWriteResult { status: "ok" })
}

pub async fn unignore_user(
    client: &Client,
    user_id: &str,
) -> Result<MatrixIgnoredUsersWriteResult, &'static str> {
    let _ = client.user_id().ok_or("v-profile.ignore-no-session")?;
    let parsed = parse_ignored_user_id(user_id)?;
    client
        .account()
        .unignore_user(&parsed)
        .await
        .map_err(|_| "v-profile.unignore-sdk-failed")?;
    Ok(MatrixIgnoredUsersWriteResult { status: "ok" })
}

fn parse_email_address(email: &str) -> Result<String, &'static str> {
    let trimmed = email.trim();
    if trimmed.is_empty() || trimmed.len() > 254 || !trimmed.contains('@') {
        return Err("v-threepid.invalid-email");
    }
    if trimmed.contains(['\n', '\r', '\0']) {
        return Err("v-threepid.invalid-email");
    }
    Ok(trimmed.to_ascii_lowercase())
}

pub struct PendingThreepid {
    client_secret: OwnedClientSecret,
    sid: OwnedSessionId,
    email: String,
    auth_session: Option<String>,
}

impl PendingThreepid {
    pub fn email(&self) -> &str {
        &self.email
    }
}

pub async fn snapshot_threepids(client: &Client) -> Result<MatrixThreepidSnapshot, &'static str> {
    let _ = client.user_id().ok_or("v-threepid.no-session")?;
    let response = client
        .account()
        .get_3pids()
        .await
        .map_err(|_| "v-threepid.snapshot-failed")?;
    let mut emails: Vec<MatrixThreepidEmail> = response
        .threepids
        .into_iter()
        .filter(|id| id.medium == Medium::Email)
        .map(|id| MatrixThreepidEmail {
            address: id.address,
        })
        .collect();
    emails.sort_by(|a, b| a.address.cmp(&b.address));
    Ok(MatrixThreepidSnapshot { emails })
}

pub async fn delete_threepid_email(
    client: &Client,
    address: &str,
) -> Result<MatrixThreepidWriteResult, &'static str> {
    let _ = client.user_id().ok_or("v-threepid.no-session")?;
    let address = parse_email_address(address)?;
    client
        .account()
        .delete_3pid(&address, Medium::Email, None)
        .await
        .map_err(|_| "v-threepid.delete-failed")?;
    Ok(MatrixThreepidWriteResult { status: "ok" })
}

pub async fn request_threepid_email_token(
    client: &Client,
    email: &str,
    pending: &Mutex<Option<PendingThreepid>>,
) -> Result<MatrixThreepidEmailTokenResult, &'static str> {
    let _ = client.user_id().ok_or("v-threepid.no-session")?;
    let email = parse_email_address(email)?;
    let client_secret = ClientSecret::new();
    let response = client
        .account()
        .request_3pid_email_token(&client_secret, &email, matrix_sdk::ruma::uint!(0))
        .await
        .map_err(|_| "v-threepid.request-failed")?;
    let session_id = response.sid.to_string();
    let mut guard = pending.lock().map_err(|_| "v-threepid.request-failed")?;
    *guard = Some(PendingThreepid {
        client_secret,
        sid: response.sid,
        email,
        auth_session: None,
    });
    Ok(MatrixThreepidEmailTokenResult { session_id })
}

pub async fn add_threepid_email(
    client: &Client,
    pending: &Mutex<Option<PendingThreepid>>,
) -> Result<MatrixThreepidAddResult, &'static str> {
    let _ = client.user_id().ok_or("v-threepid.no-session")?;
    let current = {
        let guard = pending.lock().map_err(|_| "v-threepid.add-failed")?;
        guard.as_ref().ok_or("v-threepid.not-pending")?;
        (
            guard.as_ref().expect("checked").client_secret.clone(),
            guard.as_ref().expect("checked").sid.clone(),
        )
    };
    match client
        .account()
        .add_3pid(&current.0, &current.1, None)
        .await
    {
        Ok(_) => {
            if let Ok(mut guard) = pending.lock() {
                *guard = None;
            }
            Ok(MatrixThreepidAddResult {
                status: "ok".to_owned(),
            })
        }
        Err(error) => {
            let info = error.as_uiaa_response().ok_or("v-threepid.add-failed")?;
            let session = info.session.clone().ok_or("v-threepid.add-failed")?;
            if !info
                .flows
                .iter()
                .any(|flow| flow.stages.contains(&AuthType::Password))
            {
                return Err("v-threepid.auth-unsupported");
            }
            let mut guard = pending.lock().map_err(|_| "v-threepid.add-failed")?;
            let pending_state = guard.as_mut().ok_or("v-threepid.not-pending")?;
            pending_state.auth_session = Some(session);
            Ok(MatrixThreepidAddResult {
                status: "authenticationRequired".to_owned(),
            })
        }
    }
}

pub async fn add_threepid_email_password(
    client: &Client,
    pending: &Mutex<Option<PendingThreepid>>,
    password: &str,
) -> Result<MatrixThreepidAddResult, &'static str> {
    if password.is_empty() {
        return Err("v-threepid.password-empty");
    }
    let user_id = client.user_id().ok_or("v-threepid.no-session")?;
    let current = {
        let guard = pending.lock().map_err(|_| "v-threepid.add-failed")?;
        let pending_state = guard.as_ref().ok_or("v-threepid.not-pending")?;
        (
            pending_state.client_secret.clone(),
            pending_state.sid.clone(),
            pending_state
                .auth_session
                .clone()
                .ok_or("v-threepid.not-pending")?,
        )
    };
    let mut auth = Password::new(
        UserIdentifier::Matrix(MatrixUserIdentifier::new(user_id.to_string())),
        password.to_owned(),
    );
    auth.session = Some(current.2);
    match client
        .account()
        .add_3pid(&current.0, &current.1, Some(AuthData::Password(auth)))
        .await
    {
        Ok(_) => {
            if let Ok(mut guard) = pending.lock() {
                *guard = None;
            }
            Ok(MatrixThreepidAddResult {
                status: "ok".to_owned(),
            })
        }
        Err(error) => {
            let info = error.as_uiaa_response().ok_or("v-threepid.add-failed")?;
            let session = info.session.clone().ok_or("v-threepid.add-failed")?;
            let mut guard = pending.lock().map_err(|_| "v-threepid.add-failed")?;
            let pending_state = guard.as_mut().ok_or("v-threepid.not-pending")?;
            pending_state.auth_session = Some(session);
            Err("v-threepid.add-failed")
        }
    }
}

pub fn parse_avatar_upload_mime(mime_type: &str) -> Result<Mime, &'static str> {
    let mime_type = mime_type.trim();
    if mime_type.is_empty() || mime_type.len() > 255 {
        return Err("v-send.r-avatar-upload-invalid-mime");
    }
    let parsed = mime_type
        .parse::<Mime>()
        .map_err(|_| "v-send.r-avatar-upload-invalid-mime")?;
    if parsed.type_() != mime::IMAGE {
        return Err("v-send.r-avatar-upload-invalid-mime");
    }
    Ok(parsed)
}

pub async fn upload_avatar(
    client: &Client,
    payload: Vec<u8>,
    mime_type: &str,
) -> Result<MatrixUploadAvatarResult, &'static str> {
    let _ = client
        .user_id()
        .ok_or("v-send.r-avatar-profile-no-session")?;
    if payload.is_empty() {
        return Err("v-send.r-avatar-upload-empty");
    }
    if payload.len() > MAX_AVATAR_UPLOAD_BYTES {
        return Err("v-send.r-avatar-upload-too-large");
    }
    let mime_type = parse_avatar_upload_mime(mime_type)?;
    let response = client
        .media()
        .upload(&mime_type, payload, None)
        .await
        .map_err(|_| "v-send.r-avatar-upload-sdk-failed")?;
    let mxc = response.content_uri.to_string();
    parse_own_avatar_mxc(&mxc)?;
    Ok(MatrixUploadAvatarResult { mxc })
}
