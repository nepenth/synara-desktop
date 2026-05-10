pub fn app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn revision() -> &'static str {
    option_env!("CINNY_DESKTOP_BUILD_REVISION").unwrap_or("unknown")
}

pub fn branch() -> &'static str {
    option_env!("CINNY_DESKTOP_BUILD_BRANCH").unwrap_or("unknown")
}

pub fn label() -> String {
    format!("{} {}@{}", app_version(), branch(), revision())
}

pub fn menu_label() -> String {
    format!("Build {}", label())
}
