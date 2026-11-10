//! A small status file for the world outside the window, and the
//! `synara status` command that reads it.
//!
//! The running app writes its unread, mention and approval counts to
//! `status.json` whenever they change. `synara status` prints them without
//! starting the app, so a status bar (Waybar, SketchyBar, YASB) can show them.
//! The renderer still combines the counts and pushes them with the tray
//! state; this module only publishes what it is given.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const STATUS_FILE_NAME: &str = "status.json";
const APP_DIR_NAME: &str = "synara";
const FOLLOW_POLL_INTERVAL: Duration = Duration::from_millis(500);

/// Exit code of `synara status` when the app is not running.
pub const EXIT_NOT_RUNNING: i32 = 3;
const EXIT_USAGE: i32 = 2;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DesktopStatus {
    pub running: bool,
    /// Rooms with unread messages.
    pub unread: i64,
    /// Mentions and unread direct messages.
    pub highlights: i64,
    pub pending_approvals: i64,
    pub invites: i64,
    /// Open Later items.
    pub later: i64,
    pub do_not_disturb: bool,
    pub pid: u32,
    /// Seconds since the Unix epoch.
    pub updated_at: u64,
}

impl DesktopStatus {
    /// What asks for the user: mentions, approvals and invites.
    pub fn attention(&self) -> i64 {
        self.highlights.max(0) + self.pending_approvals.max(0) + self.invites.max(0)
    }

    fn not_running() -> Self {
        Self::default()
    }
}

/// `$XDG_RUNTIME_DIR/synara/status.json` on Linux (a per-user tmpfs that is
/// cleared at logout); the user cache directory elsewhere.
pub fn status_file_path() -> Option<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        if let Some(runtime_dir) = std::env::var_os("XDG_RUNTIME_DIR").filter(|dir| !dir.is_empty())
        {
            return Some(
                PathBuf::from(runtime_dir)
                    .join(APP_DIR_NAME)
                    .join(STATUS_FILE_NAME),
            );
        }
    }
    dirs::cache_dir().map(|dir| dir.join(APP_DIR_NAME).join(STATUS_FILE_NAME))
}

fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}

fn write_status_to(path: &Path, status: &DesktopStatus) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    // Write beside the file and rename, so a reader never sees half a file.
    let staging = path.with_extension("json.tmp");
    let body = serde_json::to_vec(status).map_err(std::io::Error::other)?;
    {
        let mut file = fs::File::create(&staging)?;
        file.write_all(&body)?;
        file.write_all(b"\n")?;
    }
    fs::rename(&staging, path)
}

/// Publish the counts of the running app. Failures are logged, never fatal:
/// the status file is a convenience.
pub fn publish(mut status: DesktopStatus) {
    let Some(path) = status_file_path() else {
        return;
    };
    status.running = true;
    status.pid = std::process::id();
    status.updated_at = now_seconds();
    if let Err(error) = write_status_to(&path, &status) {
        eprintln!("[synara] Unable to write the status file: {error}");
    }
}

/// The app is exiting: say so, so a status bar stops showing old counts.
pub fn publish_not_running() {
    let Some(path) = status_file_path() else {
        return;
    };
    let mut status = DesktopStatus::not_running();
    status.updated_at = now_seconds();
    if let Err(error) = write_status_to(&path, &status) {
        eprintln!("[synara] Unable to write the status file: {error}");
    }
}

fn process_is_alive(pid: u32) -> bool {
    // After a crash the file still says "running". Linux can check cheaply;
    // elsewhere the file is trusted.
    #[cfg(target_os = "linux")]
    {
        pid != 0 && Path::new("/proc").join(pid.to_string()).exists()
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pid;
        true
    }
}

fn read_status_from(path: &Path) -> DesktopStatus {
    let Ok(body) = fs::read(path) else {
        return DesktopStatus::not_running();
    };
    let Ok(status) = serde_json::from_slice::<DesktopStatus>(&body) else {
        return DesktopStatus::not_running();
    };
    if status.running && !process_is_alive(status.pid) {
        return DesktopStatus::not_running();
    }
    status
}

fn count_phrase(count: i64, singular: &str, plural: &str) -> Option<String> {
    match count {
        value if value <= 0 => None,
        1 => Some(format!("1 {singular}")),
        value => Some(format!("{value} {plural}")),
    }
}

/// One line for a person: "2 mentions, 1 approval, 5 unread rooms".
pub fn render_text(status: &DesktopStatus) -> String {
    if !status.running {
        return "Synara is not running".to_owned();
    }
    let parts: Vec<String> = [
        count_phrase(status.highlights, "mention", "mentions"),
        count_phrase(status.pending_approvals, "approval", "approvals"),
        count_phrase(status.invites, "invite", "invites"),
        count_phrase(status.unread, "unread room", "unread rooms"),
    ]
    .into_iter()
    .flatten()
    .collect();
    if parts.is_empty() {
        "Nothing new".to_owned()
    } else {
        parts.join(", ")
    }
}

/// The most urgent state, used as Waybar's `alt` and CSS class.
pub fn status_kind(status: &DesktopStatus) -> &'static str {
    if !status.running {
        "offline"
    } else if status.pending_approvals > 0 {
        "approval"
    } else if status.highlights > 0 || status.invites > 0 {
        "mention"
    } else if status.unread > 0 {
        "unread"
    } else {
        "none"
    }
}

#[derive(Serialize)]
struct WaybarOutput {
    text: String,
    alt: &'static str,
    tooltip: String,
    class: Vec<&'static str>,
}

/// Waybar's custom-module JSON: `text` is the count that needs the user (empty
/// when there is none), `alt` picks the icon through `format-icons`.
pub fn render_waybar(status: &DesktopStatus) -> String {
    let kind = status_kind(status);
    let attention = status.attention();
    let mut class = vec![kind];
    if status.running && status.do_not_disturb {
        class.push("dnd");
    }
    let output = WaybarOutput {
        text: if status.running && attention > 0 {
            attention.to_string()
        } else {
            String::new()
        },
        alt: kind,
        tooltip: render_text(status),
        class,
    };
    serde_json::to_string(&output).unwrap_or_else(|_| "{}".to_owned())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StatusFormat {
    Text,
    Json,
    Waybar,
}

#[derive(Debug, PartialEq, Eq)]
struct StatusOptions {
    format: StatusFormat,
    follow: bool,
}

#[derive(Debug, PartialEq, Eq)]
enum CliCommand {
    Status(StatusOptions),
    StatusHelp,
    StatusUsageError(String),
}

const STATUS_USAGE: &str = "Usage: synara status [--json | --waybar] [--follow]\n\n\
Prints the running app's unread, mention and approval counts.\n\
  --json     the raw status as JSON\n\
  --waybar   JSON for a Waybar custom module (text, alt, tooltip, class)\n\
  --follow   keep running and print a line whenever the status changes\n\n\
Exits with 3 when Synara is not running (except with --follow).";

/// `args` without the program name. `None` means "not a command line for
/// this module": start the app as usual.
fn parse_cli(args: &[String]) -> Option<CliCommand> {
    if args.first().map(String::as_str) != Some("status") {
        return None;
    }
    let mut options = StatusOptions {
        format: StatusFormat::Text,
        follow: false,
    };
    for arg in &args[1..] {
        match arg.as_str() {
            "--json" => options.format = StatusFormat::Json,
            "--waybar" => options.format = StatusFormat::Waybar,
            "--follow" | "-f" => options.follow = true,
            "--help" | "-h" => return Some(CliCommand::StatusHelp),
            other => {
                return Some(CliCommand::StatusUsageError(format!(
                    "Unknown option: {other}"
                )))
            }
        }
    }
    Some(CliCommand::Status(options))
}

fn render(status: &DesktopStatus, format: StatusFormat) -> String {
    match format {
        StatusFormat::Text => render_text(status),
        StatusFormat::Json => serde_json::to_string(status).unwrap_or_else(|_| "{}".to_owned()),
        StatusFormat::Waybar => render_waybar(status),
    }
}

fn print_line(line: &str) -> bool {
    let mut stdout = std::io::stdout().lock();
    writeln!(stdout, "{line}")
        .and_then(|()| stdout.flush())
        .is_ok()
}

fn run_status(options: &StatusOptions) -> i32 {
    let Some(path) = status_file_path() else {
        eprintln!("synara: no status directory is available for this user");
        return 1;
    };
    let status = read_status_from(&path);
    if !options.follow {
        print_line(&render(&status, options.format));
        return if status.running { 0 } else { EXIT_NOT_RUNNING };
    }
    let mut last = render(&status, options.format);
    if !print_line(&last) {
        return 0;
    }
    loop {
        std::thread::sleep(FOLLOW_POLL_INTERVAL);
        let line = render(&read_status_from(&path), options.format);
        if line != last {
            // A closed pipe (the bar went away) ends the loop.
            if !print_line(&line) {
                return 0;
            }
            last = line;
        }
    }
}

/// Handle a command line that does not start the app. Returns the process
/// exit code, or `None` to start the app.
pub fn run_cli(args: &[String]) -> Option<i32> {
    match parse_cli(args)? {
        CliCommand::Status(options) => Some(run_status(&options)),
        CliCommand::StatusHelp => {
            print_line(STATUS_USAGE);
            Some(0)
        }
        CliCommand::StatusUsageError(message) => {
            eprintln!("synara: {message}\n\n{STATUS_USAGE}");
            Some(EXIT_USAGE)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn running(status: DesktopStatus) -> DesktopStatus {
        DesktopStatus {
            running: true,
            pid: std::process::id(),
            ..status
        }
    }

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn text_lists_what_needs_the_user_first() {
        let status = running(DesktopStatus {
            unread: 5,
            highlights: 2,
            pending_approvals: 1,
            ..DesktopStatus::default()
        });
        assert_eq!(
            render_text(&status),
            "2 mentions, 1 approval, 5 unread rooms"
        );
        assert_eq!(
            render_text(&running(DesktopStatus::default())),
            "Nothing new"
        );
        assert_eq!(
            render_text(&DesktopStatus::default()),
            "Synara is not running"
        );
    }

    #[test]
    fn kind_is_the_most_urgent_state() {
        let base = running(DesktopStatus::default());
        assert_eq!(status_kind(&DesktopStatus::default()), "offline");
        assert_eq!(status_kind(&base), "none");
        assert_eq!(
            status_kind(&DesktopStatus {
                unread: 1,
                ..base.clone()
            }),
            "unread"
        );
        assert_eq!(
            status_kind(&DesktopStatus {
                unread: 1,
                invites: 1,
                ..base.clone()
            }),
            "mention"
        );
        assert_eq!(
            status_kind(&DesktopStatus {
                highlights: 4,
                pending_approvals: 1,
                ..base
            }),
            "approval"
        );
    }

    #[test]
    fn waybar_output_counts_attention_and_marks_do_not_disturb() {
        let status = running(DesktopStatus {
            unread: 9,
            highlights: 2,
            pending_approvals: 1,
            do_not_disturb: true,
            ..DesktopStatus::default()
        });
        let value: serde_json::Value = serde_json::from_str(&render_waybar(&status)).unwrap();
        assert_eq!(value["text"], "3");
        assert_eq!(value["alt"], "approval");
        assert_eq!(value["class"], serde_json::json!(["approval", "dnd"]));
        assert_eq!(value["tooltip"], "2 mentions, 1 approval, 9 unread rooms");

        let quiet: serde_json::Value =
            serde_json::from_str(&render_waybar(&running(DesktopStatus {
                unread: 4,
                ..DesktopStatus::default()
            })))
            .unwrap();
        assert_eq!(quiet["text"], "");
        assert_eq!(quiet["alt"], "unread");

        let offline: serde_json::Value =
            serde_json::from_str(&render_waybar(&DesktopStatus::default())).unwrap();
        assert_eq!(offline["text"], "");
        assert_eq!(offline["class"], serde_json::json!(["offline"]));
    }

    #[test]
    fn only_the_status_subcommand_is_a_command_line() {
        assert_eq!(parse_cli(&args(&[])), None);
        // A deep link or a file path from the desktop entry starts the app.
        assert_eq!(parse_cli(&args(&["synara://room/abc"])), None);
        assert_eq!(parse_cli(&args(&["--status"])), None);
        assert_eq!(
            parse_cli(&args(&["status"])),
            Some(CliCommand::Status(StatusOptions {
                format: StatusFormat::Text,
                follow: false
            }))
        );
        assert_eq!(
            parse_cli(&args(&["status", "--waybar", "--follow"])),
            Some(CliCommand::Status(StatusOptions {
                format: StatusFormat::Waybar,
                follow: true
            }))
        );
        assert_eq!(
            parse_cli(&args(&["status", "--help"])),
            Some(CliCommand::StatusHelp)
        );
        assert!(matches!(
            parse_cli(&args(&["status", "--nope"])),
            Some(CliCommand::StatusUsageError(_))
        ));
    }

    #[test]
    fn status_file_round_trips_and_missing_or_broken_files_mean_not_running() {
        let dir = std::env::temp_dir().join(format!("synara-status-test-{}", std::process::id()));
        let path = dir.join("nested").join(STATUS_FILE_NAME);
        assert_eq!(read_status_from(&path), DesktopStatus::default());

        let status = running(DesktopStatus {
            unread: 3,
            highlights: 1,
            updated_at: 42,
            ..DesktopStatus::default()
        });
        write_status_to(&path, &status).unwrap();
        assert_eq!(read_status_from(&path), status);
        assert!(!path.with_extension("json.tmp").exists());

        fs::write(&path, b"{ not json").unwrap();
        assert_eq!(read_status_from(&path), DesktopStatus::default());

        // Fields added later must not break an older reader, nor missing
        // fields a newer one.
        fs::write(
            &path,
            format!(
                r#"{{"running":true,"unread":2,"future":1,"pid":{}}}"#,
                std::process::id()
            ),
        )
        .unwrap();
        let partial = read_status_from(&path);
        assert_eq!(partial.unread, 2);
        assert_eq!(partial.highlights, 0);

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_status_left_by_a_dead_process_reads_as_not_running() {
        let dir = std::env::temp_dir().join(format!("synara-status-stale-{}", std::process::id()));
        let path = dir.join(STATUS_FILE_NAME);
        let stale = DesktopStatus {
            running: true,
            unread: 7,
            // Above the kernel's pid limit, so never a live process.
            pid: u32::MAX,
            ..DesktopStatus::default()
        };
        write_status_to(&path, &stale).unwrap();
        assert_eq!(read_status_from(&path), DesktopStatus::default());
        let _ = fs::remove_dir_all(&dir);
    }
}
