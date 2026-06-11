use std::path::Path;
use std::process::Command;

fn git_output(repo: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?.trim().to_owned();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn sync_release_hardening_capability(manifest_dir: &Path) {
    let path = manifest_dir.join("capabilities/release-hardening.json");
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());

    println!("cargo:rerun-if-env-changed=PROFILE");

    if profile == "release" {
        let content = r#"{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "release-hardening",
  "description": "Release-only hardening: deny DevTools IPC toggle on the main webview.",
  "windows": ["main"],
  "permissions": [
    "core:webview:deny-internal-toggle-devtools"
  ]
}
"#;
        std::fs::write(&path, content).expect("write release-hardening capability");
        println!("cargo:rerun-if-changed={}", path.display());
    } else if path.exists() {
        std::fs::remove_file(&path).expect("remove release-hardening capability");
    }
}

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set");
    let manifest_path = Path::new(&manifest_dir);
    sync_release_hardening_capability(manifest_path);

    let repo = manifest_path
        .parent()
        .unwrap_or(Path::new(&manifest_dir));
    let head_path = repo.join(".git/HEAD");
    let revision = git_output(repo, &["rev-parse", "--short=12", "HEAD"])
        .unwrap_or_else(|| "unknown".to_owned());
    let branch =
        git_output(repo, &["branch", "--show-current"]).unwrap_or_else(|| "unknown".to_owned());

    println!("cargo:rerun-if-changed={}", head_path.display());
    if let Ok(head) = std::fs::read_to_string(&head_path) {
        if let Some(reference) = head.strip_prefix("ref: ").map(str::trim) {
            println!(
                "cargo:rerun-if-changed={}",
                repo.join(".git").join(reference).display()
            );
        }
    }
    println!("cargo:rustc-env=SYNARA_DESKTOP_BUILD_REVISION={revision}");
    println!("cargo:rustc-env=SYNARA_DESKTOP_BUILD_BRANCH={branch}");

    tauri_build::build()
}
