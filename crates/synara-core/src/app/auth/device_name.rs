//! Platform device display names for new Matrix devices (P3.2 / D-NEW-DEVICE).
//!
//! Product names are fixed strings — never include user ids, hostnames, or
//! secrets. Desktop cutover uses macOS/Linux; Windows and unknown targets get
//! a conservative desktop fallback so harness builds still name devices clearly.

/// Initial device display name on macOS (plan §7.1 / D-NEW-DEVICE).
pub const DEVICE_DISPLAY_NAME_MACOS: &str = "Synara macOS";

/// Initial device display name on Linux (plan §7.1 / D-NEW-DEVICE).
pub const DEVICE_DISPLAY_NAME_LINUX: &str = "Synara Linux";

/// Initial device display name on iOS (not desktop cutover; kept for shared naming table).
pub const DEVICE_DISPLAY_NAME_IOS: &str = "Synara iOS";

/// Conservative desktop fallback when the compile target is not macOS/Linux
/// (e.g. Windows CI / developer host). Still a product-shaped name, not a hostname.
pub const DEVICE_DISPLAY_NAME_DESKTOP_FALLBACK: &str = "Synara Desktop";

/// Platform used to select the initial device display name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DevicePlatform {
    MacOs,
    Linux,
    Ios,
    /// Non-macOS/Linux desktop (or unknown target).
    DesktopFallback,
}

impl DevicePlatform {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MacOs => "macos",
            Self::Linux => "linux",
            Self::Ios => "ios",
            Self::DesktopFallback => "desktop_fallback",
        }
    }

    /// Device display name for this platform (D-NEW-DEVICE).
    pub fn device_display_name(self) -> &'static str {
        match self {
            Self::MacOs => DEVICE_DISPLAY_NAME_MACOS,
            Self::Linux => DEVICE_DISPLAY_NAME_LINUX,
            Self::Ios => DEVICE_DISPLAY_NAME_IOS,
            Self::DesktopFallback => DEVICE_DISPLAY_NAME_DESKTOP_FALLBACK,
        }
    }
}

/// Compile-time host platform for product desktop builds.
///
/// - `cfg(target_os = "macos")` → [`DevicePlatform::MacOs`]
/// - `cfg(target_os = "linux")` → [`DevicePlatform::Linux`]
/// - `cfg(target_os = "ios")` → [`DevicePlatform::Ios`]
/// - otherwise → [`DevicePlatform::DesktopFallback`]
pub fn host_device_platform() -> DevicePlatform {
    #[cfg(target_os = "macos")]
    {
        DevicePlatform::MacOs
    }
    #[cfg(target_os = "linux")]
    {
        DevicePlatform::Linux
    }
    #[cfg(target_os = "ios")]
    {
        DevicePlatform::Ios
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "ios")))]
    {
        DevicePlatform::DesktopFallback
    }
}

/// Initial device display name for the host platform (D-NEW-DEVICE).
pub fn platform_device_display_name() -> &'static str {
    host_device_platform().device_display_name()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_names_match_decision_record() {
        assert_eq!(DEVICE_DISPLAY_NAME_MACOS, "Synara macOS");
        assert_eq!(DEVICE_DISPLAY_NAME_LINUX, "Synara Linux");
        assert_eq!(DEVICE_DISPLAY_NAME_IOS, "Synara iOS");
        assert_eq!(DevicePlatform::MacOs.device_display_name(), "Synara macOS");
        assert_eq!(DevicePlatform::Linux.device_display_name(), "Synara Linux");
    }

    #[test]
    fn host_name_is_product_shaped_no_secrets() {
        let name = platform_device_display_name();
        assert!(name.starts_with("Synara "), "got {name}");
        assert!(!name.contains('@'));
        assert!(!name.contains("token"));
        assert!(!name.contains("password"));
        // Host compile target must map to one of the known product strings.
        let allowed = [
            DEVICE_DISPLAY_NAME_MACOS,
            DEVICE_DISPLAY_NAME_LINUX,
            DEVICE_DISPLAY_NAME_IOS,
            DEVICE_DISPLAY_NAME_DESKTOP_FALLBACK,
        ];
        assert!(allowed.contains(&name), "unexpected host name {name}");
    }
}
