//! Live own-profile display-name / avatar writes.

use matrix_sdk::{ruma::OwnedMxcUri, Client};

use super::MatrixProfileWriteResult;

const MAX_OWN_DISPLAY_NAME_CHARS: usize = 255;

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
