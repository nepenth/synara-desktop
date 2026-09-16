//! Own-profile stream: map SDK `UserProfile` without wiping a cached pull.

use std::sync::{Arc, Mutex};

use futures_util::StreamExt;
use matrix_sdk::ruma::api::client::profile::{AvatarUrl, DisplayName};
use matrix_sdk::Client;
use tokio::task::JoinHandle;

use super::live::{get_own_profile, parse_own_avatar_mxc, parse_own_display_name};
use super::{MatrixOwnProfile, UserProfile, UserProfileIndex};

/// Tauri event when another device (or this stream) updates the own profile.
pub const OWN_PROFILE_CHANGED_EVENT: &str = "matrix-own-profile-changed";

pub type OwnProfileUpdateEmit = Arc<dyn Fn(MatrixOwnProfile) + Send + Sync>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnProfileStreamMap {
    Apply(MatrixOwnProfile),
    KeepCached,
    EmptyNeedsFallback,
    Rejected(&'static str),
}

fn field_is_empty(value: Option<&str>) -> bool {
    value.map(str::trim).unwrap_or("").is_empty()
}

/// Map stream fields onto the existing IPC DTO. An empty first item must not
/// clear a previously pulled name/avatar.
pub fn map_own_profile_stream_fields(
    user_id: &str,
    display_name: Option<&str>,
    avatar_url: Option<&str>,
    has_cached_profile: bool,
) -> OwnProfileStreamMap {
    if field_is_empty(display_name) && field_is_empty(avatar_url) {
        return if has_cached_profile {
            OwnProfileStreamMap::KeepCached
        } else {
            OwnProfileStreamMap::EmptyNeedsFallback
        };
    }
    if let Some(url) = avatar_url.map(str::trim).filter(|value| !value.is_empty()) {
        let lower = url.to_ascii_lowercase();
        if lower.starts_with("data:") || lower.starts_with("javascript:") {
            return OwnProfileStreamMap::Rejected("p6.6-forbidden-avatar-scheme");
        }
        if parse_own_avatar_mxc(url).is_err() {
            return OwnProfileStreamMap::Rejected("v-send.r-avatar-invalid-mxc");
        }
    }
    let display_name = match display_name {
        Some(name) => match parse_own_display_name(name) {
            Ok(parsed) => parsed,
            Err(err) => return OwnProfileStreamMap::Rejected(err),
        },
        None => None,
    };
    let avatar_url = match avatar_url {
        Some(url) if !url.trim().is_empty() => match parse_own_avatar_mxc(url) {
            Ok(Some(mxc)) => Some(mxc.to_string()),
            Ok(None) => None,
            Err(err) => return OwnProfileStreamMap::Rejected(err),
        },
        _ => None,
    };
    OwnProfileStreamMap::Apply(MatrixOwnProfile {
        user_id: user_id.to_owned(),
        display_name,
        avatar_url,
    })
}

fn store_own_profile(index: &Mutex<UserProfileIndex>, dto: &MatrixOwnProfile) {
    let Ok(mut index) = index.lock() else {
        return;
    };
    let _ = index.set_own_user_id(&dto.user_id);
    let _ = index.set_own_profile(UserProfile {
        user_id: dto.user_id.clone(),
        display_name: dto.display_name.clone(),
        avatar_url: dto.avatar_url.clone(),
    });
}

fn index_has_own_profile(index: &Mutex<UserProfileIndex>) -> bool {
    index
        .lock()
        .ok()
        .is_some_and(|guard| guard.own_profile().is_some())
}

/// Session-generation-scoped own-profile subscriber. Abort on owner drop.
pub struct NativeOwnProfileOwner {
    task: JoinHandle<()>,
}

impl NativeOwnProfileOwner {
    pub fn start(
        client: &Client,
        emit: OwnProfileUpdateEmit,
        session_generation: u64,
    ) -> Result<Self, &'static str> {
        if session_generation == 0 {
            return Err("v-send.r-avatar-profile-owner-invalid-generation");
        }
        let user_id = client
            .user_id()
            .ok_or("v-send.r-avatar-profile-no-session")?
            .to_string();
        let client = client.clone();
        let index = Arc::new(Mutex::new(UserProfileIndex::new(session_generation)));
        let task = tokio::spawn(async move {
            run_own_profile_stream(client, user_id, emit, index).await;
        });
        Ok(Self { task })
    }
}

impl Drop for NativeOwnProfileOwner {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn run_own_profile_stream(
    client: Client,
    user_id: String,
    emit: OwnProfileUpdateEmit,
    index: Arc<Mutex<UserProfileIndex>>,
) {
    let Ok(stream) = client.subscribe_to_own_profile() else {
        if let Ok(pulled) = get_own_profile(&client).await {
            store_own_profile(&index, &pulled);
            emit(pulled);
        }
        return;
    };
    futures_util::pin_mut!(stream);
    let mut fallback_used = false;
    while let Some(profile) = stream.next().await {
        let display_name = profile.get_static::<DisplayName>().ok().flatten();
        let avatar_url = profile
            .get_static::<AvatarUrl>()
            .ok()
            .flatten()
            .map(|mxc| mxc.to_string());
        let mapped = map_own_profile_stream_fields(
            &user_id,
            display_name.as_deref(),
            avatar_url.as_deref(),
            index_has_own_profile(&index),
        );
        match mapped {
            OwnProfileStreamMap::Apply(dto) => {
                store_own_profile(&index, &dto);
                emit(dto);
            }
            OwnProfileStreamMap::EmptyNeedsFallback if !fallback_used => {
                fallback_used = true;
                if let Ok(pulled) = get_own_profile(&client).await {
                    store_own_profile(&index, &pulled);
                    emit(pulled);
                }
            }
            OwnProfileStreamMap::KeepCached
            | OwnProfileStreamMap::EmptyNeedsFallback
            | OwnProfileStreamMap::Rejected(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_stream_does_not_clear_cached_name() {
        let cached = map_own_profile_stream_fields("@alice:example.org", None, None, true);
        assert_eq!(cached, OwnProfileStreamMap::KeepCached);
        let first = map_own_profile_stream_fields("@alice:example.org", None, None, false);
        assert_eq!(first, OwnProfileStreamMap::EmptyNeedsFallback);
        let blank = map_own_profile_stream_fields("@alice:example.org", Some("  "), Some(""), true);
        assert_eq!(blank, OwnProfileStreamMap::KeepCached);
    }

    #[test]
    fn applies_valid_mxc_and_display_name() {
        let mapped = map_own_profile_stream_fields(
            "@alice:example.org",
            Some(" Alice "),
            Some("mxc://example.org/abc"),
            false,
        );
        assert_eq!(
            mapped,
            OwnProfileStreamMap::Apply(MatrixOwnProfile {
                user_id: "@alice:example.org".into(),
                display_name: Some("Alice".into()),
                avatar_url: Some("mxc://example.org/abc".into()),
            })
        );
    }

    #[test]
    fn rejects_forbidden_avatar_scheme() {
        assert_eq!(
            map_own_profile_stream_fields(
                "@alice:example.org",
                Some("Alice"),
                Some("javascript:alert(1)"),
                false,
            ),
            OwnProfileStreamMap::Rejected("p6.6-forbidden-avatar-scheme")
        );
        assert_eq!(
            map_own_profile_stream_fields(
                "@alice:example.org",
                Some("Alice"),
                Some("data:image/png;base64,AAAA"),
                false,
            ),
            OwnProfileStreamMap::Rejected("p6.6-forbidden-avatar-scheme")
        );
    }
}
