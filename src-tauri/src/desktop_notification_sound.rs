//! Notification pings that must not register as media playback.
//!
//! HTML `<audio>` in WKWebView becomes a Now Playing session (Touch Bar
//! timeline, menu-bar playing glyph). macOS plays the bundled WAV through
//! `NSSound` instead. Other platforms return false so the renderer can keep
//! its existing `<audio>` fallback.

use tauri::{AppHandle, Runtime};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotificationSoundKind {
    Message,
    Invite,
}

impl NotificationSoundKind {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "message" | "notification" => Ok(Self::Message),
            "invite" => Ok(Self::Invite),
            other => Err(format!("unknown notification sound: {other}")),
        }
    }

    pub fn wav_bytes(self) -> &'static [u8] {
        match self {
            Self::Message => include_bytes!("../sounds/notification.wav"),
            Self::Invite => include_bytes!("../sounds/invite.wav"),
        }
    }
}

pub fn play_notification_sound<R: Runtime>(app: &AppHandle<R>, kind: &str) -> Result<bool, String> {
    let kind = NotificationSoundKind::parse(kind)?;
    #[cfg(target_os = "macos")]
    {
        play_macos_nssound(app, kind.wav_bytes())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, kind);
        Ok(false)
    }
}

#[cfg(target_os = "macos")]
fn play_macos_nssound<R: Runtime>(app: &AppHandle<R>, wav: &'static [u8]) -> Result<bool, String> {
    if objc2::MainThreadMarker::new().is_some() {
        return Ok(play_nssound_on_main(wav));
    }

    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    app.run_on_main_thread(move || {
        let _ = tx.send(play_nssound_on_main(wav));
    })
    .map_err(|error| error.to_string())?;
    rx.recv().map_err(|error| error.to_string())
}

#[cfg(target_os = "macos")]
fn play_nssound_on_main(wav: &'static [u8]) -> bool {
    use std::cell::RefCell;

    use objc2::rc::Retained;
    use objc2::AnyThread;
    use objc2_app_kit::NSSound;
    use objc2_foundation::NSData;

    thread_local! {
        static CURRENT_SOUND: RefCell<Option<Retained<NSSound>>> = const { RefCell::new(None) };
    }

    let data = NSData::with_bytes(wav);
    let Some(sound) = NSSound::initWithData(NSSound::alloc(), &data) else {
        eprintln!("[synara] failed to decode notification sound");
        return false;
    };
    if !sound.play() {
        eprintln!("[synara] failed to play notification sound");
        return false;
    }
    CURRENT_SOUND.with(|slot| {
        *slot.borrow_mut() = Some(sound);
    });
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sound_kind_parses_product_names() {
        assert_eq!(
            NotificationSoundKind::parse("message").unwrap(),
            NotificationSoundKind::Message
        );
        assert_eq!(
            NotificationSoundKind::parse("Notification").unwrap(),
            NotificationSoundKind::Message
        );
        assert_eq!(
            NotificationSoundKind::parse(" invite ").unwrap(),
            NotificationSoundKind::Invite
        );
        assert!(NotificationSoundKind::parse("ringtone").is_err());
    }

    #[test]
    fn bundled_notification_sounds_are_pcm_wav() {
        for bytes in [
            NotificationSoundKind::Message.wav_bytes(),
            NotificationSoundKind::Invite.wav_bytes(),
        ] {
            assert!(bytes.starts_with(b"RIFF"));
            assert_eq!(&bytes[8..12], b"WAVE");
            assert!(bytes.len() > 44);
        }
    }

    #[test]
    fn macos_playback_reports_decode_and_play_result() {
        let source = include_str!("desktop_notification_sound.rs");
        let macos = source
            .split("fn play_macos_nssound")
            .nth(1)
            .unwrap_or("")
            .split("fn play_nssound_on_main")
            .next()
            .unwrap_or("");
        assert!(macos.contains("Result<bool, String>"));
        assert!(source.contains("play_macos_nssound(app, kind.wav_bytes())"));
        assert!(source.contains("fn play_nssound_on_main(wav: &'static [u8]) -> bool"));
        assert!(source.contains("return false;"));
    }
}
