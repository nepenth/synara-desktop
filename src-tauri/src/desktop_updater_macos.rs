//! macOS in-app update install that keeps the installed bundle's identity.
//!
//! `tauri-plugin-updater` 2.13.2 installs an update by exchanging the whole
//! `/Applications/Synara.app` directory for a freshly extracted one. The path
//! and bundle ID stay the same, but the directory is a new file, so the Dock's
//! bookmark of the pinned app resolves as stale and the Dock shows a second
//! icon for the running app. Swapping only `Contents` inside the existing
//! bundle directory keeps that directory, and with it the Dock tile.
//!
//! The download still goes through the plugin's `Update::download`, which
//! verifies the minisign signature before any byte reaches the disk. When the
//! Contents swap cannot run (another file system, no write access, a bundle
//! with more than `Contents`), the plugin's own `Update::install` runs instead,
//! which keeps its administrator prompt.

use serde::Serialize;
use tauri::ipc::Channel;

/// The download events the renderer already handles for the plugin's
/// `downloadAndInstall` (`DesktopUpdateDownloadEvent` in `desktopUpdater.ts`).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", content = "data")]
// Only the macOS install sends these; other targets keep the type for the
// command signature.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub enum DesktopUpdateDownloadEvent {
    #[serde(rename_all = "camelCase")]
    Started {
        content_length: Option<u64>,
    },
    #[serde(rename_all = "camelCase")]
    Progress {
        chunk_length: usize,
    },
    Finished,
}

/// Downloads, verifies and installs the update the renderer prompted for. The
/// renderer relaunches the app once this returns.
#[tauri::command]
pub async fn desktop_updater_install(
    app: tauri::AppHandle,
    expected_version: String,
    on_event: Channel<DesktopUpdateDownloadEvent>,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        macos::download_and_install(app, expected_version, on_event).await
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, expected_version, on_event);
        Err("desktop_updater_install is only available on macOS.".to_owned())
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::DesktopUpdateDownloadEvent;
    use std::ffi::{c_void, CString};
    use std::io;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::MetadataExt;
    use std::path::{Component, Path, PathBuf};
    use tauri::ipc::Channel;
    use tauri_plugin_updater::UpdaterExt;

    const CONTENTS: &str = "Contents";

    pub(super) async fn download_and_install(
        app: tauri::AppHandle,
        expected_version: String,
        on_event: Channel<DesktopUpdateDownloadEvent>,
    ) -> Result<(), String> {
        let update = app
            .updater()
            .map_err(|error| error.to_string())?
            .check()
            .await
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "No update is available any more.".to_owned())?;
        if update.version != expected_version {
            return Err(format!(
                "The available update changed from {expected_version} to {}. Check for updates again.",
                update.version
            ));
        }

        let mut started = false;
        let progress = on_event.clone();
        // `download` verifies the signature and returns only verified bytes.
        let bytes = update
            .download(
                move |chunk_length, content_length| {
                    if !started {
                        started = true;
                        let _ =
                            progress.send(DesktopUpdateDownloadEvent::Started { content_length });
                    }
                    let _ = progress.send(DesktopUpdateDownloadEvent::Progress { chunk_length });
                },
                || {
                    let _ = on_event.send(DesktopUpdateDownloadEvent::Finished);
                },
            )
            .await
            .map_err(|error| error.to_string())?;

        let app_path = std::env::current_exe()
            .map_err(|error| error.to_string())
            .and_then(|exe| {
                tauri_plugin_updater::extract_path_from_executable(&exe)
                    .map_err(|error| error.to_string())
            })?;

        tauri::async_runtime::spawn_blocking(move || {
            install_preserving_identity(&app_path, &bytes, swap_paths, |bytes| {
                update.install(bytes).map_err(|error| error.to_string())
            })?;
            refresh_bundle_registration(&app_path);
            Ok(())
        })
        .await
        .map_err(|error| error.to_string())?
    }

    /// How the update was installed.
    #[derive(Debug, PartialEq, Eq)]
    pub(super) enum InstallOutcome {
        /// `Contents` was swapped inside the existing bundle directory.
        ContentsSwapped,
        /// The plugin's whole-bundle install ran, for the reason given.
        Fallback(String),
    }

    /// Installs the verified update archive `bytes` into the bundle at `app`
    /// by swapping its `Contents` with the archive's, and runs `fallback`
    /// (the plugin's install) when that cannot be done.
    pub(super) fn install_preserving_identity(
        app: &Path,
        bytes: &[u8],
        swap: impl FnOnce(&Path, &Path) -> io::Result<()>,
        fallback: impl FnOnce(&[u8]) -> Result<(), String>,
    ) -> Result<InstallOutcome, String> {
        let run_fallback = |reason: String| {
            eprintln!("[synara] installing the update by replacing the bundle: {reason}");
            fallback(bytes).map(|()| InstallOutcome::Fallback(reason))
        };

        if let Err(reason) = installed_bundle_supports_contents_swap(app) {
            return run_fallback(reason);
        }
        let tmp_root = match tmp_root(app) {
            Ok(dir) => dir,
            Err(error) => return run_fallback(error.to_string()),
        };
        let staging = match tempfile::Builder::new()
            .prefix("synara_updated_contents")
            .tempdir_in(&tmp_root)
        {
            Ok(dir) => dir,
            Err(error) => return run_fallback(format!("cannot create a staging dir: {error}")),
        };

        extract_bundle(bytes, staging.path())
            .map_err(|error| format!("cannot unpack the update: {error}"))?;
        if let Err(reason) = top_level_matches(app, staging.path()) {
            drop(staging);
            return run_fallback(reason);
        }

        let staged_contents = staging.path().join(CONTENTS);
        match swap(&staged_contents, &app.join(CONTENTS)) {
            Ok(()) => {}
            Err(error) if swap_error_needs_fallback(&error) => {
                drop(staging);
                return run_fallback(format!("cannot swap Contents: {error}"));
            }
            Err(error) => return Err(format!("cannot install the update: {error}")),
        }

        // The staging dir now holds the previous Contents.
        if let Err(error) = staging.close() {
            eprintln!("[synara] could not remove the previous app contents: {error}");
        }
        Ok(InstallOutcome::ContentsSwapped)
    }

    /// The installed app must be a real `.app` directory holding only
    /// `Contents`, so swapping `Contents` gives the same result as replacing
    /// the bundle.
    fn installed_bundle_supports_contents_swap(app: &Path) -> Result<(), String> {
        let meta = std::fs::symlink_metadata(app)
            .map_err(|error| format!("cannot read {}: {error}", app.display()))?;
        if !meta.is_dir() {
            return Err(format!("{} is not a directory", app.display()));
        }
        if app.extension().and_then(|ext| ext.to_str()) != Some("app") {
            return Err(format!("{} is not an app bundle", app.display()));
        }
        let contents = std::fs::symlink_metadata(app.join(CONTENTS))
            .map_err(|error| format!("cannot read the bundle's Contents: {error}"))?;
        if !contents.is_dir() {
            return Err("the bundle's Contents is not a directory".to_owned());
        }
        Ok(())
    }

    /// Both the installed bundle and the update must hold exactly `Contents`.
    fn top_level_matches(app: &Path, staged: &Path) -> Result<(), String> {
        for (label, dir) in [("installed app", app), ("update", staged)] {
            let names = top_level_names(dir).map_err(|error| error.to_string())?;
            if names != [CONTENTS] {
                return Err(format!(
                    "the {label} has more than Contents at its top level: {names:?}"
                ));
            }
        }
        let staged_contents =
            std::fs::symlink_metadata(staged.join(CONTENTS)).map_err(|error| error.to_string())?;
        if !staged_contents.is_dir() {
            return Err("the update's Contents is not a directory".to_owned());
        }
        Ok(())
    }

    fn top_level_names(dir: &Path) -> io::Result<Vec<String>> {
        let mut names = std::fs::read_dir(dir)?
            .map(|entry| entry.map(|entry| entry.file_name().to_string_lossy().into_owned()))
            .collect::<io::Result<Vec<_>>>()?;
        names.sort();
        Ok(names)
    }

    /// Renames only work within one file system, so the staging dir goes in the
    /// system temp dir when it is on the app's volume and next to the app
    /// otherwise, as the plugin does.
    pub(super) fn tmp_root(app: &Path) -> io::Result<PathBuf> {
        let app_dev = std::fs::symlink_metadata(app)?.dev();
        [
            Some(std::env::temp_dir()),
            app.parent().map(Path::to_path_buf),
        ]
        .into_iter()
        .flatten()
        .find(|dir| dir.metadata().is_ok_and(|meta| meta.dev() == app_dev))
        .ok_or_else(|| io::Error::other("no temp dir on the same volume as the app"))
    }

    /// Unpacks the updater `.app.tar.gz` into `dest`, dropping the archive's
    /// top-level `<App>.app` component as the plugin does. The archive is
    /// already signature-verified; entries that would land outside `dest`
    /// (absolute paths, `..`, or paths through a symlink) and hard links are
    /// still refused.
    pub(super) fn extract_bundle(bytes: &[u8], dest: &Path) -> io::Result<()> {
        let decoder = flate2::read::GzDecoder::new(io::Cursor::new(bytes));
        let mut archive = tar::Archive::new(decoder);
        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?.into_owned();
            let relative = relative_entry_path(&path)?;
            let Some(relative) = relative else {
                continue;
            };
            if entry.header().entry_type().is_hard_link() {
                return Err(invalid(format!(
                    "hard link in the update: {}",
                    path.display()
                )));
            }
            let target = dest.join(&relative);
            ensure_no_symlink_ancestor(dest, &relative)?;
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            entry.unpack(&target)?;
        }
        Ok(())
    }

    /// The entry's path below the top-level component, or `None` for the
    /// top-level directory itself.
    fn relative_entry_path(path: &Path) -> io::Result<Option<PathBuf>> {
        let mut parts = Vec::new();
        for component in path.components() {
            match component {
                Component::Normal(part) => parts.push(part),
                Component::CurDir => {}
                Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                    return Err(invalid(format!(
                        "update entry escapes the bundle: {}",
                        path.display()
                    )));
                }
            }
        }
        if parts.len() <= 1 {
            return Ok(None);
        }
        Ok(Some(parts[1..].iter().collect()))
    }

    /// A symlink unpacked earlier must not redirect a later entry outside `dest`.
    fn ensure_no_symlink_ancestor(dest: &Path, relative: &Path) -> io::Result<()> {
        let mut current = dest.to_path_buf();
        let mut components = relative.components().peekable();
        while let Some(component) = components.next() {
            if components.peek().is_none() {
                break;
            }
            current.push(component);
            match std::fs::symlink_metadata(&current) {
                Ok(meta) if meta.file_type().is_symlink() => {
                    return Err(invalid(format!(
                        "update entry goes through a symlink: {}",
                        relative.display()
                    )));
                }
                Ok(_) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => break,
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    fn invalid(message: String) -> io::Error {
        io::Error::new(io::ErrorKind::InvalidData, message)
    }

    /// Errors for which the plugin's install is the right next step: the file
    /// system cannot swap (`ENOTSUP` on HFS+, others elsewhere), the paths are
    /// on different volumes, or the app needs administrator rights.
    pub(super) fn swap_error_needs_fallback(error: &io::Error) -> bool {
        error.kind() == io::ErrorKind::PermissionDenied
            || matches!(
                error.raw_os_error(),
                Some(
                    libc::ENOTSUP
                        | libc::EOPNOTSUPP
                        | libc::ENOSYS
                        | libc::EINVAL
                        | libc::EXDEV
                        | libc::EPERM
                        | libc::EACCES
                        | libc::EROFS
                )
            )
    }

    /// Exchanges `staged` and `target` in one step with `renamex_np(RENAME_SWAP)`.
    pub(super) fn swap_paths(staged: &Path, target: &Path) -> io::Result<()> {
        let from = CString::new(staged.as_os_str().as_bytes())?;
        let to = CString::new(target.as_os_str().as_bytes())?;
        // SAFETY: both pointers are valid NUL-terminated strings that outlive
        // the call, and `renamex_np` only reads them.
        let result = unsafe { libc::renamex_np(from.as_ptr(), to.as_ptr(), libc::RENAME_SWAP) };
        if result == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    /// Updates the bundle's modification time and re-registers it with
    /// LaunchServices so the new Info.plist and version are picked up.
    fn refresh_bundle_registration(app: &Path) {
        if let Err(error) = touch(app) {
            eprintln!("[synara] could not update the app bundle's modification time: {error}");
        }
        let status = register_with_launch_services(app);
        if status != 0 {
            eprintln!(
                "[synara] LSRegisterURL returned {status} for {}",
                app.display()
            );
        }
    }

    fn touch(path: &Path) -> io::Result<()> {
        std::fs::File::open(path)?.set_modified(std::time::SystemTime::now())
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFURLCreateFromFileSystemRepresentation(
            allocator: *const c_void,
            buffer: *const u8,
            buffer_length: isize,
            is_directory: u8,
        ) -> *const c_void;
        fn CFRelease(value: *const c_void);
    }

    #[link(name = "CoreServices", kind = "framework")]
    extern "C" {
        fn LSRegisterURL(url: *const c_void, update: u8) -> i32;
    }

    fn register_with_launch_services(app: &Path) -> i32 {
        let bytes = app.as_os_str().as_bytes();
        let Ok(length) = isize::try_from(bytes.len()) else {
            return -1;
        };
        // SAFETY: `bytes` is valid for `length` bytes during the call; a null
        // allocator selects the default one. The returned URL is released below.
        let url = unsafe {
            CFURLCreateFromFileSystemRepresentation(std::ptr::null(), bytes.as_ptr(), length, 1)
        };
        if url.is_null() {
            return -1;
        }
        // SAFETY: `url` is a valid CFURL owned by this function.
        let status = unsafe { LSRegisterURL(url, 1) };
        // SAFETY: `url` came from a Create function and is released once.
        unsafe { CFRelease(url) };
        status
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::cell::Cell;

        fn tar_gz(entries: &[(&str, &[u8])]) -> Vec<u8> {
            let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
            let mut builder = tar::Builder::new(encoder);
            for (path, data) in entries {
                let mut header = tar::Header::new_gnu();
                header.set_size(data.len() as u64);
                header.set_mode(0o755);
                header.set_entry_type(tar::EntryType::Regular);
                // `set_path` refuses `..` and absolute paths, so write the
                // name bytes directly to build hostile archives.
                let name = &mut header.as_old_mut().name;
                name[..path.len()].copy_from_slice(path.as_bytes());
                header.set_cksum();
                builder.append(&header, *data).unwrap();
            }
            builder.into_inner().unwrap().finish().unwrap()
        }

        fn update_archive(tag: &str) -> Vec<u8> {
            let plist = format!("<plist>new {tag}</plist>");
            tar_gz(&[
                ("Synara.app/Contents/Info.plist", plist.as_bytes()),
                ("Synara.app/Contents/MacOS/synara", b"new binary"),
            ])
        }

        /// A temp dir holding an installed `Synara.app` whose Contents carry
        /// `tag`, so a test can find its own leftovers.
        fn installed_app(tag: &str) -> (tempfile::TempDir, PathBuf) {
            let root = tempfile::tempdir().unwrap();
            let app = root.path().join("Synara.app");
            std::fs::create_dir_all(app.join("Contents/MacOS")).unwrap();
            std::fs::write(app.join("Contents/Info.plist"), "<plist>old</plist>").unwrap();
            std::fs::write(app.join("Contents/MacOS/synara"), "old binary").unwrap();
            std::fs::write(app.join("Contents/old-only"), tag).unwrap();
            (root, app)
        }

        /// Staging dirs left in the temp root holding this test's old or new Contents.
        fn leftover_staging(app: &Path, tag: &str) -> Vec<PathBuf> {
            let new_plist = format!("<plist>new {tag}</plist>");
            std::fs::read_dir(tmp_root(app).unwrap())
                .unwrap()
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.starts_with("synara_updated_contents"))
                })
                .filter(|path| {
                    std::fs::read_to_string(path.join("Contents/old-only"))
                        .is_ok_and(|old| old == tag)
                        || std::fs::read_to_string(path.join("Contents/Info.plist"))
                            .is_ok_and(|plist| plist == new_plist)
                })
                .collect()
        }

        #[test]
        fn the_contents_swap_installs_the_new_contents_in_the_same_bundle_dir() {
            let tag = "swap-installs";
            let (_root, app) = installed_app(tag);
            let inode = std::fs::metadata(&app).unwrap().ino();

            let outcome =
                install_preserving_identity(&app, &update_archive(tag), swap_paths, |_| {
                    panic!("fallback must not run")
                })
                .unwrap();

            assert_eq!(outcome, InstallOutcome::ContentsSwapped);
            assert_eq!(
                std::fs::read_to_string(app.join("Contents/Info.plist")).unwrap(),
                format!("<plist>new {tag}</plist>")
            );
            assert_eq!(
                std::fs::read(app.join("Contents/MacOS/synara")).unwrap(),
                b"new binary"
            );
            assert_eq!(std::fs::metadata(&app).unwrap().ino(), inode);
            // The old Contents went to the staging dir, which is removed.
            assert!(!app.join("Contents/old-only").exists());
            assert!(leftover_staging(&app, tag).is_empty());
        }

        #[test]
        fn extraction_skips_the_top_level_bundle_component() {
            let dest = tempfile::tempdir().unwrap();
            extract_bundle(&update_archive("extract"), dest.path()).unwrap();
            assert_eq!(top_level_names(dest.path()).unwrap(), ["Contents"]);
            assert!(dest.path().join("Contents/MacOS/synara").is_file());
        }

        #[test]
        fn extraction_refuses_parent_dir_entries() {
            let outer = tempfile::tempdir().unwrap();
            let dest = outer.path().join("staging");
            std::fs::create_dir(&dest).unwrap();
            let archive = tar_gz(&[("Synara.app/Contents/../../escaped", b"x")]);
            let error = extract_bundle(&archive, &dest).unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::InvalidData);
            assert!(!outer.path().join("escaped").exists());
            assert!(top_level_names(&dest).unwrap().is_empty());
        }

        #[test]
        fn extraction_refuses_absolute_entries() {
            let dest = tempfile::tempdir().unwrap();
            let archive = tar_gz(&[("/tmp/Synara.app/Contents/x", b"x")]);
            let error = extract_bundle(&archive, dest.path()).unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::InvalidData);
            assert!(top_level_names(dest.path()).unwrap().is_empty());
        }

        #[test]
        fn the_fallback_runs_when_the_file_system_cannot_swap() {
            let tag = "enotsup";
            let (_root, app) = installed_app(tag);
            let fallback_ran = Cell::new(false);

            let outcome = install_preserving_identity(
                &app,
                &update_archive(tag),
                |_, _| Err(io::Error::from_raw_os_error(libc::ENOTSUP)),
                |bytes| {
                    assert!(!bytes.is_empty());
                    fallback_ran.set(true);
                    Ok(())
                },
            )
            .unwrap();

            assert!(fallback_ran.get());
            assert!(matches!(outcome, InstallOutcome::Fallback(_)));
            // Nothing was swapped, and the staging dir is gone.
            assert_eq!(
                std::fs::read_to_string(app.join("Contents/old-only")).unwrap(),
                tag
            );
            assert!(leftover_staging(&app, tag).is_empty());
        }

        #[test]
        fn the_fallback_runs_for_permission_errors_but_not_other_swap_errors() {
            assert!(swap_error_needs_fallback(&io::Error::from(
                io::ErrorKind::PermissionDenied
            )));
            assert!(swap_error_needs_fallback(&io::Error::from_raw_os_error(
                libc::EXDEV
            )));
            assert!(!swap_error_needs_fallback(&io::Error::from_raw_os_error(
                libc::ENOENT
            )));
        }

        #[test]
        fn the_fallback_runs_when_the_bundle_holds_more_than_contents() {
            let (_root, app) = installed_app("extra-top-level");
            std::fs::write(app.join("Icon\r"), "").unwrap();
            let fallback_ran = Cell::new(false);

            let outcome = install_preserving_identity(
                &app,
                &update_archive("extra-top-level"),
                |_, _| panic!("swap must not run"),
                |_| {
                    fallback_ran.set(true);
                    Ok(())
                },
            )
            .unwrap();

            assert!(fallback_ran.get());
            assert!(matches!(outcome, InstallOutcome::Fallback(_)));
        }
    }
}
