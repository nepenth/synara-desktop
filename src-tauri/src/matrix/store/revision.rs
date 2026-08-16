//! Non-destructive store revision migration + explicit reset recovery.
//!
//! The Matrix SDK SQLite state/crypto store is encrypted using a Keychain-backed
//! key. A build that changes the on-disk schema, store layout, or key contract
//! must migrate that store **before** `Client::builder().build()` opens SQLite.
//!
//! This module is deliberately conservative:
//! - migration is forward-only, deterministic, and writes a non-secret manifest;
//! - a bad/ahead manifest is *reset-required*, never auto-wiped;
//! - explicit reset archives local store dirs before recreating them, preserving
//!   evidence/recovery material rather than silently deleting it;
//! - diagnostics are static identifiers only — no paths, key ids, key bytes,
//!   homeserver URLs, tokens, or raw SDK/Keychain text.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use synara_core::app::store::{StorePathError, StorePaths};

/// Current on-disk layout revision. Bump this whenever a change touches state,
/// crypto, cache, media, or store-key schema; add the matching migration step.
pub const STORE_LAYOUT_VERSION: u32 = 1;
/// Oldest known layout. A pre-manifest store is interpreted as layout 0.
pub const MIN_MIGRATABLE_LAYOUT_VERSION: u32 = 0;
/// Non-secret revision sidecar in the account root.
pub const STORE_REVISION_MANIFEST_FILE: &str = "revision.json";
/// Archive directory for explicitly reset store content.
pub const STORE_RECOVERY_ARCHIVE_SEGMENT: &str = "recovery";
/// Static link/schema marker.
pub const MATRIX_STORE_REVISION_MARKER: &str = "matrix-store-revision-migrate-reset-v1";

/// Non-secret persisted revision state.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreRevisionManifest {
    /// Opaque account-root segment; binds a copied manifest to its own account.
    pub account_segment: String,
    /// Last successfully applied store layout revision.
    pub layout_version: u32,
    /// Source revision for the most recent forward migration (optional on reset).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_version: Option<u32>,
}

/// Only static/non-sensitive migration failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreMigrationError {
    CorruptManifest,
    ManifestAccountMismatch,
    RevisionAhead { observed: u32, known: u32 },
    MigrationGap { from: u32, expected: u32 },
    StepFailed { step_id: &'static str },
    Io { kind: &'static str },
}

impl StoreMigrationError {
    /// User/support-safe recovery diagnostic. This id is intentionally coarse:
    /// whether a reset is safe must never depend on exposed local error detail.
    pub fn diagnostic_id(&self) -> &'static str {
        match self {
            Self::CorruptManifest | Self::ManifestAccountMismatch => {
                "p3.2-login-store-reset-required"
            }
            Self::RevisionAhead { .. } | Self::MigrationGap { .. } => {
                "p3.2-login-store-migration-required"
            }
            Self::StepFailed { .. } | Self::Io { .. } => "p3.2-login-store-migration-failed",
        }
    }
}

impl std::fmt::Display for StoreMigrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never embed raw filesystem / SDK / Keychain data here.
        write!(
            f,
            "store revision operation failed ({})",
            self.diagnostic_id()
        )
    }
}
impl std::error::Error for StoreMigrationError {}
impl From<StorePathError> for StoreMigrationError {
    fn from(error: StorePathError) -> Self {
        match error {
            StorePathError::Io(_) => Self::Io {
                kind: "store-path-io",
            },
            StorePathError::PathEscapesRoot => Self::Io {
                kind: "store-path-escape",
            },
            StorePathError::RelativeAppDataRoot => Self::Io {
                kind: "store-path-relative-root",
            },
            StorePathError::SymlinkRefused => Self::Io {
                kind: "store-path-symlink",
            },
        }
    }
}

/// Successful pre-open revision state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreRevisionDecision {
    UpToDate {
        layout_version: u32,
    },
    Migrated {
        from: u32,
        to: u32,
        steps: Vec<&'static str>,
    },
}

/// An explicitly requested reset result. No paths are surfaced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoreResetOutcome {
    pub archived_components: u8,
    pub layout_version: u32,
}

/// A deterministic revision step. Each new store-schema revision must add a
/// contiguous step; this guard makes accidental un-migratable version bumps fail
/// closed rather than mislabel an old store as current.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoreMigrationStep {
    pub from: u32,
    pub to: u32,
    pub id: &'static str,
}

/// Baseline migration: legacy stores had no manifest. It deliberately changes
/// no user data; it establishes the revision record before SQLite opens.
const MIGRATION_STEPS: &[StoreMigrationStep] = &[StoreMigrationStep {
    from: 0,
    to: 1,
    id: "r1-revision-manifest-baseline",
}];

/// Migrate a store to [`STORE_LAYOUT_VERSION`] before SDK SQLite open.
///
/// Safe for fresh accounts and legacy pre-manifest stores. This never deletes
/// data. Callers must surface [`StoreMigrationError::diagnostic_id`] and offer
/// an explicit reset only when this returns an error.
pub fn migrate_store_to_current(
    paths: &StorePaths,
) -> Result<StoreRevisionDecision, StoreMigrationError> {
    paths.ensure_dirs()?;
    let from = match load_manifest(paths)? {
        None => MIN_MIGRATABLE_LAYOUT_VERSION,
        Some(manifest) => {
            if manifest.account_segment != paths.account_segment() {
                return Err(StoreMigrationError::ManifestAccountMismatch);
            }
            if manifest.layout_version > STORE_LAYOUT_VERSION {
                return Err(StoreMigrationError::RevisionAhead {
                    observed: manifest.layout_version,
                    known: STORE_LAYOUT_VERSION,
                });
            }
            manifest.layout_version
        }
    };

    if from == STORE_LAYOUT_VERSION {
        return Ok(StoreRevisionDecision::UpToDate {
            layout_version: STORE_LAYOUT_VERSION,
        });
    }

    let steps = migration_chain(from)?;
    for step in &steps {
        apply_step(paths, *step)?;
    }
    write_manifest(
        paths,
        &StoreRevisionManifest {
            account_segment: paths.account_segment().to_owned(),
            layout_version: STORE_LAYOUT_VERSION,
            previous_version: Some(from),
        },
    )?;
    Ok(StoreRevisionDecision::Migrated {
        from,
        to: STORE_LAYOUT_VERSION,
        steps: steps.iter().map(|step| step.id).collect(),
    })
}

/// Explicitly archive local SDK store directories then create an empty current
/// layout. **Never call this automatically.** It preserves the Keychain key;
/// callers that intentionally need a new key must separately delete/regenerate
/// it only after user confirmation.
pub fn reset_store_for_recovery(
    paths: &StorePaths,
) -> Result<StoreResetOutcome, StoreMigrationError> {
    // Refuse every managed layout symlink before creating an archive or moving
    // data. In particular, `recovery/` must never redirect reset output
    // outside the account root.
    paths.ensure_dirs()?;
    let manifest = manifest_path(paths);
    let archive_manifest = validate_recovery_reset_sources(paths, &manifest)?;
    let archive = create_archive_dir(paths)?;
    let mut components = vec![
        (paths.state_dir(), "state"),
        (paths.crypto_dir(), "crypto"),
        (paths.cache_dir(), "cache"),
        (paths.media_dir(), "media"),
    ];
    // A malformed/ahead manifest is itself recovery evidence. Move it before
    // writing the reset manifest so recovery never silently overwrites it.
    if archive_manifest {
        components.push((&manifest, STORE_REVISION_MANIFEST_FILE));
    }

    let mut moved: Vec<(PathBuf, PathBuf)> = Vec::with_capacity(components.len());
    for (source, label) in components {
        let target = archive.join(label);
        if let Err(error) = fs::rename(source, &target) {
            // Best-effort rollback; never report paths/raw errors.
            for (original, archived) in moved.iter().rev() {
                let _ = fs::rename(archived, original);
            }
            return Err(io_error(error));
        }
        moved.push((source.to_path_buf(), target));
    }
    // Recreate hardened directories using the existing symlink/root checks.
    paths.ensure_dirs()?;
    write_manifest(
        paths,
        &StoreRevisionManifest {
            account_segment: paths.account_segment().to_owned(),
            layout_version: STORE_LAYOUT_VERSION,
            previous_version: None,
        },
    )?;
    Ok(StoreResetOutcome {
        archived_components: moved.len() as u8,
        layout_version: STORE_LAYOUT_VERSION,
    })
}

fn migration_chain(from: u32) -> Result<Vec<StoreMigrationStep>, StoreMigrationError> {
    let mut current = from;
    let mut steps = Vec::new();
    while current < STORE_LAYOUT_VERSION {
        let Some(step) = MIGRATION_STEPS
            .iter()
            .copied()
            .find(|step| step.from == current)
        else {
            return Err(StoreMigrationError::MigrationGap {
                from: current,
                expected: current.saturating_add(1),
            });
        };
        if step.to <= step.from || step.to > STORE_LAYOUT_VERSION {
            return Err(StoreMigrationError::StepFailed { step_id: step.id });
        }
        current = step.to;
        steps.push(step);
    }
    Ok(steps)
}

fn apply_step(_paths: &StorePaths, step: StoreMigrationStep) -> Result<(), StoreMigrationError> {
    match step.id {
        // v1 only establishes the manifest. Future schema changes must add
        // exact, idempotent filesystem/SQLite preparation here.
        "r1-revision-manifest-baseline" => Ok(()),
        _ => Err(StoreMigrationError::StepFailed { step_id: step.id }),
    }
}

fn load_manifest(paths: &StorePaths) -> Result<Option<StoreRevisionManifest>, StoreMigrationError> {
    match fs::read(manifest_path(paths)) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|_| StoreMigrationError::CorruptManifest),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(io_error(error)),
    }
}

fn write_manifest(
    paths: &StorePaths,
    manifest: &StoreRevisionManifest,
) -> Result<(), StoreMigrationError> {
    let target = manifest_path(paths);
    let temporary = paths.account_root().join(format!(
        ".{STORE_REVISION_MANIFEST_FILE}.tmp-{}",
        std::process::id()
    ));
    let bytes = serde_json::to_vec_pretty(manifest).map_err(|_| StoreMigrationError::Io {
        kind: "manifest-encode",
    })?;
    // Never follow a pre-existing predictable temporary symlink. `create_new`
    // makes a collision fail closed rather than letting fs::write overwrite an
    // external file through such a link.
    let mut temporary_file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(io_error)?;
    temporary_file.write_all(&bytes).map_err(io_error)?;
    drop(temporary_file);
    fs::rename(&temporary, target).map_err(io_error)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(manifest_path(paths), fs::Permissions::from_mode(0o600))
            .map_err(io_error)?;
    }
    Ok(())
}

/// Verify the reset inputs before creating an archive. The revision manifest
/// must be a regular local file when present, and every pre-existing archive
/// entry must be non-symlinked so recovery never follows or writes through an
/// attacker-controlled link.
fn validate_recovery_reset_sources(
    paths: &StorePaths,
    manifest: &Path,
) -> Result<bool, StoreMigrationError> {
    validate_recovery_archive_root(&paths.account_root().join(STORE_RECOVERY_ARCHIVE_SEGMENT))?;
    match fs::symlink_metadata(manifest) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(unsafe_recovery_component()),
        Ok(metadata) if !metadata.file_type().is_file() => Err(unsafe_recovery_component()),
        Ok(_) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(io_error(error)),
    }
}

fn validate_recovery_archive_root(root: &Path) -> Result<(), StoreMigrationError> {
    match fs::symlink_metadata(root) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(unsafe_recovery_component())
        }
        Ok(metadata) if !metadata.is_dir() => return Err(unsafe_recovery_component()),
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(io_error(error)),
    }

    for entry in fs::read_dir(root).map_err(io_error)? {
        let entry = entry.map_err(io_error)?;
        if fs::symlink_metadata(entry.path())
            .map_err(io_error)?
            .file_type()
            .is_symlink()
        {
            return Err(unsafe_recovery_component());
        }
    }
    Ok(())
}

fn unsafe_recovery_component() -> StoreMigrationError {
    StoreMigrationError::Io {
        kind: "recovery-unsafe-component",
    }
}

fn create_archive_dir(paths: &StorePaths) -> Result<PathBuf, StoreMigrationError> {
    let root = paths.account_root().join(STORE_RECOVERY_ARCHIVE_SEGMENT);
    validate_recovery_archive_root(&root)?;
    match fs::create_dir(&root) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(io_error(error)),
    }
    // Recheck after mkdir so a raced/pre-existing symlink can never be used as
    // the archive root. `paths.ensure_dirs` has already created account_root,
    // so this intentionally never uses create_dir_all (which follows links).
    validate_recovery_archive_root(&root)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).map_err(io_error)?;
    }
    let base = format!("reset-v{}-{}", STORE_LAYOUT_VERSION, now_unix_ms());
    for suffix in 0..32_u8 {
        let candidate = root.join(format!("{base}-{suffix}"));
        match fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(io_error(error)),
        }
    }
    Err(StoreMigrationError::Io {
        kind: "recovery-archive-collision",
    })
}

fn manifest_path(paths: &StorePaths) -> PathBuf {
    paths.account_root().join(STORE_REVISION_MANIFEST_FILE)
}

fn io_error(error: io::Error) -> StoreMigrationError {
    let kind = match error.kind() {
        io::ErrorKind::NotFound => "not-found",
        io::ErrorKind::PermissionDenied => "permission-denied",
        io::ErrorKind::AlreadyExists => "already-exists",
        io::ErrorKind::InvalidData => "invalid-data",
        io::ErrorKind::Interrupted => "interrupted",
        _ => "io",
    };
    StoreMigrationError::Io { kind }
}

fn now_unix_ms() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

/// Static marker for link/schema smoke.
pub fn matrix_store_revision_marker() -> &'static str {
    debug_assert_eq!(STORE_LAYOUT_VERSION, 1);
    MATRIX_STORE_REVISION_MARKER
}

#[cfg(test)]
mod revision_tests {
    use super::*;
    use crate::matrix::store::AccountIdentity;

    fn root(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "synara-store-revision-{label}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }
    fn identity() -> AccountIdentity {
        AccountIdentity::new("@alice:example.org", "https://example.org").unwrap()
    }
    fn paths(root: &Path) -> StorePaths {
        StorePaths::derive(root, &identity()).unwrap()
    }

    #[test]
    fn missing_manifest_migrates_legacy_layout_and_is_idempotent() {
        let root = root("migrate");
        let paths = paths(&root);
        let first = migrate_store_to_current(&paths).unwrap();
        assert_eq!(
            first,
            StoreRevisionDecision::Migrated {
                from: 0,
                to: STORE_LAYOUT_VERSION,
                steps: vec!["r1-revision-manifest-baseline"],
            }
        );
        assert_eq!(
            migrate_store_to_current(&paths).unwrap(),
            StoreRevisionDecision::UpToDate { layout_version: 1 }
        );
    }

    #[test]
    fn corrupt_or_copied_manifest_fails_closed_with_static_reset_id() {
        let root = root("bad-manifest");
        let paths = paths(&root);
        paths.ensure_dirs().unwrap();
        fs::write(manifest_path(&paths), b"not json").unwrap();
        let corrupt = migrate_store_to_current(&paths).unwrap_err();
        assert_eq!(corrupt.diagnostic_id(), "p3.2-login-store-reset-required");
        fs::write(
            manifest_path(&paths),
            serde_json::to_vec(&StoreRevisionManifest {
                account_segment: "v1_someone_else".into(),
                layout_version: 1,
                previous_version: None,
            })
            .unwrap(),
        )
        .unwrap();
        let mismatch = migrate_store_to_current(&paths).unwrap_err();
        assert_eq!(mismatch, StoreMigrationError::ManifestAccountMismatch);
        assert_eq!(mismatch.diagnostic_id(), "p3.2-login-store-reset-required");
    }

    #[test]
    fn ahead_revision_fails_closed_and_never_downgrades() {
        let root = root("ahead");
        let paths = paths(&root);
        paths.ensure_dirs().unwrap();
        write_manifest(
            &paths,
            &StoreRevisionManifest {
                account_segment: paths.account_segment().into(),
                layout_version: 2,
                previous_version: Some(1),
            },
        )
        .unwrap();
        let error = migrate_store_to_current(&paths).unwrap_err();
        assert!(matches!(
            error,
            StoreMigrationError::RevisionAhead {
                observed: 2,
                known: 1
            }
        ));
        assert_eq!(error.diagnostic_id(), "p3.2-login-store-migration-required");
    }

    #[test]
    fn explicit_reset_archives_data_and_manifest_then_rebuilds_layout() {
        let root = root("reset");
        let paths = paths(&root);
        migrate_store_to_current(&paths).unwrap();
        let original_manifest = fs::read(manifest_path(&paths)).unwrap();
        fs::write(paths.state_dir().join("state.db"), b"opaque-state").unwrap();
        fs::write(paths.cache_dir().join("cache.db"), b"opaque-cache").unwrap();

        let outcome = reset_store_for_recovery(&paths).unwrap();
        assert_eq!(outcome.archived_components, 5);
        assert_eq!(outcome.layout_version, STORE_LAYOUT_VERSION);
        assert!(!paths.state_dir().join("state.db").exists());
        assert!(!paths.cache_dir().join("cache.db").exists());
        assert!(paths.state_dir().is_dir());
        assert!(paths.cache_dir().is_dir());
        assert_eq!(
            migrate_store_to_current(&paths).unwrap(),
            StoreRevisionDecision::UpToDate { layout_version: 1 }
        );
        let archive_root = paths.account_root().join(STORE_RECOVERY_ARCHIVE_SEGMENT);
        let archive = fs::read_dir(&archive_root)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        for component in ["state", "crypto", "cache", "media"] {
            assert!(archive.join(component).is_dir(), "{component} is archived");
        }
        assert_eq!(
            fs::read(archive.join(STORE_REVISION_MANIFEST_FILE)).unwrap(),
            original_manifest,
            "the pre-reset revision evidence is archived, never overwritten"
        );
        assert_ne!(
            fs::read(manifest_path(&paths)).unwrap(),
            original_manifest,
            "the rebuilt layout has its own reset manifest"
        );
    }

    #[cfg(unix)]
    #[test]
    fn explicit_reset_refuses_recovery_symlinks_without_external_write() {
        use std::os::unix::fs::symlink;

        for (label, install_link) in [
            ("recovery-root-symlink", true),
            ("existing-archive-symlink", false),
        ] {
            let root = root(label);
            let paths = paths(&root);
            paths.ensure_dirs().unwrap();
            let outside = std::env::temp_dir().join(format!(
                "synara-store-revision-{label}-outside-{}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&outside);
            fs::create_dir_all(&outside).unwrap();
            let recovery = paths.account_root().join(STORE_RECOVERY_ARCHIVE_SEGMENT);
            if install_link {
                symlink(&outside, &recovery).unwrap();
            } else {
                fs::create_dir(&recovery).unwrap();
                symlink(&outside, recovery.join("prior-archive")).unwrap();
            }

            let error = reset_store_for_recovery(&paths).unwrap_err();
            assert_eq!(error.diagnostic_id(), "p3.2-login-store-migration-failed");
            assert_eq!(
                fs::read_dir(&outside).unwrap().count(),
                0,
                "recovery must not write through {label}"
            );
            let _ = fs::remove_dir_all(&outside);
        }
    }

    #[cfg(unix)]
    #[test]
    fn explicit_reset_refuses_manifest_symlink_before_archiving_or_overwriting() {
        use std::os::unix::fs::symlink;

        let root = root("manifest-symlink");
        let paths = paths(&root);
        paths.ensure_dirs().unwrap();
        let outside = std::env::temp_dir().join(format!(
            "synara-store-revision-manifest-outside-{}",
            std::process::id()
        ));
        let _ = fs::remove_file(&outside);
        fs::write(&outside, b"outside-manifest").unwrap();
        symlink(&outside, manifest_path(&paths)).unwrap();

        let error = reset_store_for_recovery(&paths).unwrap_err();
        assert_eq!(error.diagnostic_id(), "p3.2-login-store-migration-failed");
        assert_eq!(fs::read(&outside).unwrap(), b"outside-manifest");
        assert!(
            !paths
                .account_root()
                .join(STORE_RECOVERY_ARCHIVE_SEGMENT)
                .exists(),
            "an unsafe manifest must fail before any archive is created"
        );
        let _ = fs::remove_file(&outside);
    }

    #[test]
    fn manifest_is_secret_and_path_free() {
        let value = serde_json::to_string(&StoreRevisionManifest {
            account_segment: "v1_alice_opaque".into(),
            layout_version: 1,
            previous_version: Some(0),
        })
        .unwrap();
        for forbidden in [
            "token",
            "access",
            "refresh",
            "password",
            "passphrase",
            "key",
            "/Users/",
        ] {
            assert!(!value
                .to_ascii_lowercase()
                .contains(&forbidden.to_ascii_lowercase()));
        }
    }

    #[test]
    fn marker_is_stable() {
        assert_eq!(matrix_store_revision_marker(), MATRIX_STORE_REVISION_MARKER);
    }
}
