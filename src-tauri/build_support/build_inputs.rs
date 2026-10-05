//! Build inputs shared with dependency-free regression tests.

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

const RELEASE_HARDENING: &str = r#"{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "release-hardening",
  "description": "Release-only hardening: deny DevTools IPC toggle on the main webview.",
  "windows": ["main"],
  "permissions": [
    "core:webview:deny-internal-toggle-devtools"
  ]
}
"#;

pub fn sync_release_hardening_capability(manifest_dir: &Path, profile: &str) -> io::Result<()> {
    let path = manifest_dir.join("capabilities/release-hardening.json");
    if profile == "release" {
        match fs::read(&path) {
            Ok(content) if content == RELEASE_HARDENING.as_bytes() => return Ok(()),
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        fs::write(path, RELEASE_HARDENING)
    } else {
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        }
    }
}

pub fn git_output(repo: &Path, args: &[&str]) -> Option<String> {
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
    (!value.is_empty()).then_some(value)
}

fn git_path(repo: &Path, name: &str) -> Option<PathBuf> {
    let path = PathBuf::from(git_output(repo, &["rev-parse", "--git-path", name])?);
    Some(if path.is_absolute() {
        path
    } else {
        repo.join(path)
    })
}

fn existing_path(path: &Path) -> Option<PathBuf> {
    fs::canonicalize(path).ok()
}

/// Git resolves worktree-local HEAD and shared refs for us. Cargo considers a
/// missing watched path changed on every run, so a packed branch watches the
/// nearest existing ref directory until Git creates its loose ref again.
pub fn git_watch_paths(repo: &Path) -> Vec<PathBuf> {
    let mut paths = BTreeSet::new();
    if repo.join(".git").is_file() {
        if let Some(path) = existing_path(&repo.join(".git")) {
            paths.insert(path);
        }
    }
    if let Some(head) = git_path(repo, "HEAD").and_then(|path| existing_path(&path)) {
        paths.insert(head);
    }
    if let Some(reference) = git_output(repo, &["symbolic-ref", "--quiet", "HEAD"]) {
        if let Some(mut path) = git_path(repo, &reference) {
            loop {
                if let Some(existing) = existing_path(&path) {
                    paths.insert(existing);
                    break;
                }
                if !path.pop() {
                    break;
                }
            }
        }
        if let Some(packed) = git_path(repo, "packed-refs").and_then(|path| existing_path(&path)) {
            paths.insert(packed);
        }
    }
    paths.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_TEMP: AtomicUsize = AtomicUsize::new(0);

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "synara-build-inputs-{}-{stamp}-{}",
                std::process::id(),
                NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    fn git(repo: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_AUTHOR_NAME", "Build input test")
            .env("GIT_AUTHOR_EMAIL", "build-input@example.invalid")
            .env("GIT_COMMITTER_NAME", "Build input test")
            .env("GIT_COMMITTER_EMAIL", "build-input@example.invalid")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }

    fn repository(root: &TempDir) -> PathBuf {
        let repo = root.0.join("repo");
        fs::create_dir(&repo).unwrap();
        git(&repo, &["init", "--initial-branch=main"]);
        git(
            &repo,
            &[
                "-c",
                "commit.gpgsign=false",
                "commit",
                "--allow-empty",
                "-m",
                "initial",
            ],
        );
        repo
    }

    #[test]
    fn unchanged_release_capability_preserves_mtime_and_profile_change_removes_it() {
        let root = TempDir::new();
        fs::create_dir(root.0.join("capabilities")).unwrap();
        let path = root.0.join("capabilities/release-hardening.json");
        sync_release_hardening_capability(&root.0, "release").unwrap();
        // Give the file a known old mtime, so even a fast accidental rewrite is
        // caught without sleeps or assumptions about filesystem resolution.
        let old = UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000);
        fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_times(fs::FileTimes::new().set_modified(old))
            .unwrap();
        let before = fs::metadata(&path).unwrap().modified().unwrap();
        sync_release_hardening_capability(&root.0, "release").unwrap();
        assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), before);
        assert_eq!(fs::read(&path).unwrap(), RELEASE_HARDENING.as_bytes());
        fs::write(&path, "stale capability").unwrap();
        sync_release_hardening_capability(&root.0, "release").unwrap();
        assert_eq!(fs::read(&path).unwrap(), RELEASE_HARDENING.as_bytes());
        sync_release_hardening_capability(&root.0, "debug").unwrap();
        assert!(!path.exists());
        sync_release_hardening_capability(&root.0, "debug").unwrap();
    }

    #[test]
    fn ordinary_checkout_watches_head_and_loose_ref_and_updates_revision() {
        let root = TempDir::new();
        let repo = repository(&root);
        let paths = git_watch_paths(&repo);
        assert!(paths.contains(&repo.join(".git/HEAD").canonicalize().unwrap()));
        assert!(paths.contains(&repo.join(".git/refs/heads/main").canonicalize().unwrap()));
        assert!(paths.iter().all(|path| path.exists()));
        let before = git_output(&repo, &["rev-parse", "--short=12", "HEAD"]).unwrap();
        git(
            &repo,
            &[
                "-c",
                "commit.gpgsign=false",
                "commit",
                "--allow-empty",
                "-m",
                "second",
            ],
        );
        let after = git_output(&repo, &["rev-parse", "--short=12", "HEAD"]).unwrap();
        assert_ne!(before, after);
        assert_eq!(
            git_output(&repo, &["branch", "--show-current"]).unwrap(),
            "main"
        );
    }

    #[test]
    fn linked_worktree_watches_its_head_and_shared_branch_ref() {
        let root = TempDir::new();
        let repo = repository(&root);
        let worktree = root.0.join("linked");
        git(
            &repo,
            &[
                "worktree",
                "add",
                "-b",
                "feature",
                worktree.to_str().unwrap(),
            ],
        );
        assert!(worktree.join(".git").is_file());
        let paths = git_watch_paths(&worktree);
        assert!(paths.contains(
            &repo
                .join(".git/worktrees/linked/HEAD")
                .canonicalize()
                .unwrap()
        ));
        assert!(paths.contains(&repo.join(".git/refs/heads/feature").canonicalize().unwrap()));
        assert!(paths.iter().all(|path| path.exists()));
        assert_eq!(
            git_output(&worktree, &["branch", "--show-current"]).unwrap(),
            "feature"
        );
    }

    #[test]
    fn packed_branch_watches_packed_refs_and_loose_ref_creation_parent() {
        let root = TempDir::new();
        let repo = repository(&root);
        git(&repo, &["pack-refs", "--all", "--prune"]);
        assert!(!repo.join(".git/refs/heads/main").exists());
        let paths = git_watch_paths(&repo);
        assert!(paths.contains(&repo.join(".git/packed-refs").canonicalize().unwrap()));
        assert!(paths.contains(&repo.join(".git/refs/heads").canonicalize().unwrap()));
        assert!(paths.iter().all(|path| path.exists()));
        let before = git_output(&repo, &["rev-parse", "--short=12", "HEAD"]).unwrap();
        git(
            &repo,
            &[
                "-c",
                "commit.gpgsign=false",
                "commit",
                "--allow-empty",
                "-m",
                "unpack branch",
            ],
        );
        assert_ne!(
            before,
            git_output(&repo, &["rev-parse", "--short=12", "HEAD"]).unwrap()
        );
        assert!(git_watch_paths(&repo)
            .contains(&repo.join(".git/refs/heads/main").canonicalize().unwrap()));
    }

    #[test]
    fn detached_head_watches_existing_head_without_missing_ref_inputs() {
        let root = TempDir::new();
        let repo = repository(&root);
        git(&repo, &["checkout", "--detach"]);
        assert_eq!(
            git_watch_paths(&repo),
            vec![repo.join(".git/HEAD").canonicalize().unwrap()]
        );
        assert!(git_output(&repo, &["branch", "--show-current"]).is_none());
    }
}
