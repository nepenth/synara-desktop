//! Account-scoped X.509 identity trust (experimental, default off).
//!
//! Persist enablement and imported CA PEMs under `{account_root}/x509/`.
//! Injection into `ClientBuilder` happens only when the setting is on **and**
//! at least one trust anchor parses. SAS / own-device verification is unchanged.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const X509_DIR: &str = "x509";
const SETTINGS_FILE: &str = "settings.json";
const TRUST_ANCHORS_FILE: &str = "trust-anchors.pem";
const SIGNER_CERT_FILE: &str = "signer-cert.pem";
const SIGNER_KEY_FILE: &str = "signer-key.pem";
const APPLIED_FILE: &str = "applied.json";

const MAX_PEM_BYTES: usize = 256 * 1024;
const MAX_SIGNER_KEY_BYTES: usize = 32 * 1024;
const MAX_CA_CERTS: usize = 16;
#[cfg(feature = "x509-identity")]
const MAX_CERT_VERIFIED_IDENTITIES: usize = 64;
#[cfg(feature = "x509-identity")]
const MAX_JOINED_ROOMS_SCANNED: usize = 512;

/// Algorithm id on an MSK `signatures` map (CMS SignedData).
pub const X509_SIGNATURE_ALGORITHM: &str = "io.element.x509";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum X509StoreError {
    Io,
    InvalidPem,
    TooLarge,
    EmptySelection,
    CaNotFound,
}

impl X509StoreError {
    pub fn diagnostic_id(self) -> &'static str {
        match self {
            Self::Io => "v-crypto.x509-store-unavailable",
            Self::InvalidPem => "v-crypto.x509-invalid-pem",
            Self::TooLarge => "v-crypto.x509-pem-too-large",
            Self::EmptySelection => "v-crypto.x509-empty-selection",
            Self::CaNotFound => "v-crypto.x509-ca-not-found",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SettingsFile {
    #[serde(default)]
    enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppliedFile {
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    ca_fingerprints: Vec<String>,
    #[serde(default)]
    verifier_configured: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeX509CaSummary {
    pub fingerprint: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeX509IdentityStatus {
    pub enabled: bool,
    pub has_ca: bool,
    pub cas: Vec<NativeX509CaSummary>,
    pub signer_imported: bool,
    pub verifier_configured: bool,
    pub reload_required: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub certificate_verified_identities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct X509Runtime {
    pub enabled: bool,
    pub trust_anchors_pem: String,
    pub cas: Vec<NativeX509CaSummary>,
    pub signer_imported: bool,
    pub signer_cert_pem: Option<String>,
    pub signer_key_pem: Option<String>,
}

impl X509Runtime {
    pub fn default_off() -> Self {
        Self {
            enabled: false,
            trust_anchors_pem: String::new(),
            cas: Vec::new(),
            signer_imported: false,
            signer_cert_pem: None,
            signer_key_pem: None,
        }
    }

    pub fn has_ca(&self) -> bool {
        !self.cas.is_empty()
    }

    /// Inject only when the user turned the setting on *and* a CA parses.
    pub fn should_inject_verifier(&self) -> bool {
        self.enabled && pem_parses_as_trust_anchors(&self.trust_anchors_pem)
    }
}

fn x509_dir(account_root: &Path) -> PathBuf {
    account_root.join(X509_DIR)
}

fn confine_under(account_root: &Path, candidate: &Path) -> Result<(), X509StoreError> {
    if candidate == account_root || candidate.starts_with(account_root) {
        Ok(())
    } else {
        Err(X509StoreError::Io)
    }
}

fn ensure_x509_dir(account_root: &Path) -> Result<PathBuf, X509StoreError> {
    let dir = x509_dir(account_root);
    confine_under(account_root, &dir)?;
    fs::create_dir_all(&dir).map_err(|_| X509StoreError::Io)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))
            .map_err(|_| X509StoreError::Io)?;
    }
    Ok(dir)
}

fn read_optional(path: &Path, max_bytes: usize) -> Result<Option<String>, X509StoreError> {
    match fs::read(path) {
        Ok(bytes) if bytes.len() > max_bytes => Err(X509StoreError::TooLarge),
        Ok(bytes) => Ok(Some(
            String::from_utf8(bytes).map_err(|_| X509StoreError::InvalidPem)?,
        )),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err(X509StoreError::Io),
    }
}

fn write_secret_file(path: &Path, contents: &str) -> Result<(), X509StoreError> {
    fs::write(path, contents).map_err(|_| X509StoreError::Io)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .map_err(|_| X509StoreError::Io)?;
    }
    Ok(())
}

pub fn load_runtime(account_root: &Path) -> X509Runtime {
    load_runtime_result(account_root).unwrap_or_else(|_| X509Runtime::default_off())
}

fn load_runtime_result(account_root: &Path) -> Result<X509Runtime, X509StoreError> {
    let dir = x509_dir(account_root);
    let settings = match read_optional(&dir.join(SETTINGS_FILE), 4 * 1024)? {
        Some(raw) => serde_json::from_str::<SettingsFile>(&raw).unwrap_or_default(),
        None => SettingsFile::default(),
    };
    let trust_anchors_pem =
        read_optional(&dir.join(TRUST_ANCHORS_FILE), MAX_PEM_BYTES)?.unwrap_or_default();
    let cas = summarize_cas(&trust_anchors_pem);
    let signer_cert_pem = read_optional(&dir.join(SIGNER_CERT_FILE), MAX_PEM_BYTES)?;
    let signer_key_pem = read_optional(&dir.join(SIGNER_KEY_FILE), MAX_SIGNER_KEY_BYTES)?;
    let signer_imported = signer_cert_pem
        .as_ref()
        .is_some_and(|pem| looks_like_pem(pem))
        && signer_key_pem
            .as_ref()
            .is_some_and(|pem| looks_like_private_key(pem));
    Ok(X509Runtime {
        enabled: settings.enabled,
        trust_anchors_pem,
        cas,
        signer_imported,
        signer_cert_pem: signer_cert_pem.filter(|pem| looks_like_pem(pem)),
        signer_key_pem: signer_key_pem.filter(|pem| looks_like_private_key(pem)),
    })
}

pub fn status_from_store(
    account_root: &Path,
    certificate_verified_identities: Vec<String>,
) -> NativeX509IdentityStatus {
    let runtime = load_runtime(account_root);
    let applied = load_applied(account_root);
    let verifier_live = applied
        .as_ref()
        .map(|applied| applied.verifier_configured)
        .unwrap_or(false);
    let desired_inject = runtime.should_inject_verifier();
    let fingerprints: Vec<String> = runtime
        .cas
        .iter()
        .map(|ca| ca.fingerprint.clone())
        .collect();
    let reload_required = match applied {
        Some(applied) => {
            applied.verifier_configured != desired_inject
                || applied.enabled != runtime.enabled
                || applied.ca_fingerprints != fingerprints
        }
        None => desired_inject,
    };
    NativeX509IdentityStatus {
        enabled: runtime.enabled,
        has_ca: runtime.has_ca(),
        cas: runtime.cas,
        signer_imported: runtime.signer_imported,
        verifier_configured: verifier_live,
        reload_required,
        certificate_verified_identities,
    }
}

pub fn set_enabled(account_root: &Path, enabled: bool) -> Result<X509Runtime, X509StoreError> {
    let dir = ensure_x509_dir(account_root)?;
    let path = dir.join(SETTINGS_FILE);
    confine_under(account_root, &path)?;
    write_secret_file(
        &path,
        &serde_json::to_string(&SettingsFile { enabled }).map_err(|_| X509StoreError::Io)?,
    )?;
    Ok(load_runtime(account_root))
}

pub fn import_ca_pem(account_root: &Path, pem: &str) -> Result<X509Runtime, X509StoreError> {
    if pem.len() > MAX_PEM_BYTES {
        return Err(X509StoreError::TooLarge);
    }
    let incoming = iter_pem_certificates(pem);
    if incoming.is_empty() {
        return Err(X509StoreError::InvalidPem);
    }
    if !pem_parses_as_trust_anchors(pem) {
        return Err(X509StoreError::InvalidPem);
    }
    let dir = ensure_x509_dir(account_root)?;
    let path = dir.join(TRUST_ANCHORS_FILE);
    confine_under(account_root, &path)?;
    let existing = read_optional(&path, MAX_PEM_BYTES)?.unwrap_or_default();
    let mut blocks = iter_pem_certificates(&existing);
    for cert in incoming {
        if !blocks
            .iter()
            .any(|have| fingerprint_cert(have) == fingerprint_cert(&cert))
        {
            blocks.push(cert);
        }
    }
    if blocks.len() > MAX_CA_CERTS {
        return Err(X509StoreError::TooLarge);
    }
    let combined = join_pem_certificates(&blocks);
    if !pem_parses_as_trust_anchors(&combined) {
        return Err(X509StoreError::InvalidPem);
    }
    write_secret_file(&path, &combined)?;
    Ok(load_runtime(account_root))
}

pub fn remove_ca(account_root: &Path, fingerprint: &str) -> Result<X509Runtime, X509StoreError> {
    let runtime = load_runtime(account_root);
    let blocks = iter_pem_certificates(&runtime.trust_anchors_pem);
    let kept: Vec<String> = blocks
        .into_iter()
        .filter(|block| fingerprint_cert(block) != fingerprint)
        .collect();
    if kept.len() == iter_pem_certificates(&runtime.trust_anchors_pem).len() {
        return Err(X509StoreError::CaNotFound);
    }
    let dir = ensure_x509_dir(account_root)?;
    let path = dir.join(TRUST_ANCHORS_FILE);
    confine_under(account_root, &path)?;
    if kept.is_empty() {
        let _ = fs::remove_file(&path);
    } else {
        write_secret_file(&path, &join_pem_certificates(&kept))?;
    }
    Ok(load_runtime(account_root))
}

pub fn import_signer_pems(
    account_root: &Path,
    cert_pem: &str,
    key_pem: &str,
) -> Result<X509Runtime, X509StoreError> {
    if cert_pem.len() > MAX_PEM_BYTES || key_pem.len() > MAX_SIGNER_KEY_BYTES {
        return Err(X509StoreError::TooLarge);
    }
    if iter_pem_certificates(cert_pem).is_empty() || !looks_like_private_key(key_pem) {
        return Err(X509StoreError::InvalidPem);
    }
    #[cfg(feature = "x509-identity")]
    {
        if matrix_sdk_crypto::x509::RustRawX509Signer::new_from_pem_data(cert_pem, key_pem).is_err()
        {
            return Err(X509StoreError::InvalidPem);
        }
    }
    let dir = ensure_x509_dir(account_root)?;
    let cert_path = dir.join(SIGNER_CERT_FILE);
    let key_path = dir.join(SIGNER_KEY_FILE);
    confine_under(account_root, &cert_path)?;
    confine_under(account_root, &key_path)?;
    write_secret_file(&cert_path, cert_pem)?;
    write_secret_file(&key_path, key_pem)?;
    Ok(load_runtime(account_root))
}

pub fn record_applied(account_root: &Path, verifier_configured: bool) {
    let dir = x509_dir(account_root);
    if !dir.exists() && !verifier_configured {
        return;
    }
    let runtime = load_runtime(account_root);
    let Ok(dir) = ensure_x509_dir(account_root) else {
        return;
    };
    let applied = AppliedFile {
        enabled: runtime.enabled,
        ca_fingerprints: runtime
            .cas
            .iter()
            .map(|ca| ca.fingerprint.clone())
            .collect(),
        verifier_configured,
    };
    if let Ok(json) = serde_json::to_string(&applied) {
        let _ = write_secret_file(&dir.join(APPLIED_FILE), &json);
    }
}

fn load_applied(account_root: &Path) -> Option<AppliedFile> {
    let raw = read_optional(&x509_dir(account_root).join(APPLIED_FILE), 8 * 1024).ok()??;
    serde_json::from_str(&raw).ok()
}

pub fn looks_like_pem(value: &str) -> bool {
    value.contains("-----BEGIN CERTIFICATE-----") && value.contains("-----END CERTIFICATE-----")
}

fn looks_like_private_key(value: &str) -> bool {
    (value.contains("-----BEGIN PRIVATE KEY-----") && value.contains("-----END PRIVATE KEY-----"))
        || (value.contains("-----BEGIN RSA PRIVATE KEY-----")
            && value.contains("-----END RSA PRIVATE KEY-----"))
}

pub fn json_contains_secret_material(json: &str) -> bool {
    let lower = json.to_ascii_lowercase();
    lower.contains("begin certificate")
        || lower.contains("begin private")
        || lower.contains("-----begin")
        || lower.contains("cms")
        || lower.contains("\"pem\"")
        || lower.contains("privatekey")
        || lower.contains("private_key")
}

fn iter_pem_certificates(pem: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = pem;
    const BEGIN: &str = "-----BEGIN CERTIFICATE-----";
    const END: &str = "-----END CERTIFICATE-----";
    while let Some(start) = rest.find(BEGIN) {
        let from_begin = &rest[start..];
        let Some(end) = from_begin.find(END) else {
            break;
        };
        out.push(from_begin[..end + END.len()].to_owned());
        rest = &from_begin[end + END.len()..];
    }
    out
}

fn join_pem_certificates(blocks: &[String]) -> String {
    let mut out = String::new();
    for block in blocks {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(block.trim());
        out.push('\n');
    }
    out
}

fn fingerprint_cert(block: &str) -> String {
    let body: String = block
        .lines()
        .filter(|line| !line.starts_with("-----"))
        .collect();
    let digest = Sha256::digest(body.as_bytes());
    format!("sha256:{}", hex_lower(&digest))
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn summarize_cas(pem: &str) -> Vec<NativeX509CaSummary> {
    iter_pem_certificates(pem)
        .into_iter()
        .map(|block| {
            let fingerprint = fingerprint_cert(&block);
            let short = fingerprint
                .strip_prefix("sha256:")
                .unwrap_or(&fingerprint)
                .get(..8)
                .unwrap_or("ca");
            NativeX509CaSummary {
                label: format!("Imported CA {short}"),
                fingerprint,
            }
        })
        .collect()
}

fn pem_parses_as_trust_anchors(pem: &str) -> bool {
    if iter_pem_certificates(pem).is_empty() {
        return false;
    }
    #[cfg(feature = "x509-identity")]
    {
        ensure_aws_lc_rustls_provider();
        matrix_sdk_crypto::x509::RustRawX509Verifier::new_from_pem_data(pem).is_ok()
    }
    #[cfg(not(feature = "x509-identity"))]
    {
        true
    }
}

/// Install aws-lc as the rustls process default. `rust-x509-verifier-impl`
/// enables rustls `ring` but uses `CryptoProvider::get_default()`; never
/// install ring here.
#[cfg(feature = "x509-identity")]
pub fn ensure_aws_lc_rustls_provider() {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
}

/// Whether `ClientBuilder::with_x509_verifier(Some(_))` should run.
pub fn should_inject_verifier(account_root: &Path) -> bool {
    load_runtime(account_root).should_inject_verifier()
}

#[cfg(feature = "x509-identity")]
pub fn build_verifier(
    pem: &str,
) -> Option<std::sync::Arc<dyn matrix_sdk_crypto::x509::RawX509Verifier>> {
    ensure_aws_lc_rustls_provider();
    matrix_sdk_crypto::x509::RustRawX509Verifier::new_from_pem_data(pem)
        .ok()
        .map(|verifier| {
            std::sync::Arc::new(verifier)
                as std::sync::Arc<dyn matrix_sdk_crypto::x509::RawX509Verifier>
        })
}

#[cfg(feature = "x509-identity")]
pub fn build_signer(
    cert_pem: &str,
    key_pem: &str,
) -> Option<std::sync::Arc<dyn matrix_sdk_crypto::x509::RawX509Signer>> {
    matrix_sdk_crypto::x509::RustRawX509Signer::new_from_pem_data(cert_pem, key_pem)
        .ok()
        .map(|signer| {
            std::sync::Arc::new(signer)
                as std::sync::Arc<dyn matrix_sdk_crypto::x509::RawX509Signer>
        })
}

pub fn master_key_has_x509_signature(master_json: &str) -> bool {
    master_json.contains(X509_SIGNATURE_ALGORITHM)
}

#[cfg(feature = "x509-identity")]
pub async fn list_certificate_verified_user_ids(client: &matrix_sdk::Client) -> Vec<String> {
    use matrix_sdk::encryption::identities::UserIdentity;
    use matrix_sdk::ruma::events::room::member::MembershipState;
    use matrix_sdk::RoomMemberships;

    let Some(own_id) = client.user_id().map(|id| id.to_owned()) else {
        return Vec::new();
    };
    let encryption = client.encryption();
    let own_verified = match encryption.get_user_identity(&own_id).await {
        Ok(Some(own)) => own.is_verified(),
        _ => false,
    };

    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for room in client
        .joined_rooms()
        .into_iter()
        .take(MAX_JOINED_ROOMS_SCANNED)
    {
        let Ok(members) = room.members_no_sync(RoomMemberships::JOIN).await else {
            continue;
        };
        for member in members {
            if member.membership() != &MembershipState::Join {
                continue;
            }
            let user_id = member.user_id().to_owned();
            if user_id == own_id || !seen.insert(user_id.clone()) {
                continue;
            }
            let Ok(Some(identity)) = encryption.get_user_identity(&user_id).await else {
                continue;
            };
            let _: &UserIdentity = &identity;
            if !identity.is_verified() {
                continue;
            }
            let master_json = format!("{:?}", identity.master_key());
            let has_x509 = master_key_has_x509_signature(&master_json);
            // Own-unverified + is_verified implies X.509 (USK path needs own latch).
            if has_x509 || !own_verified {
                out.push(user_id.to_string());
                if out.len() >= MAX_CERT_VERIFIED_IDENTITIES {
                    return out;
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    const TEST_CA_PEM: &str = include_str!("test_ca.pem");

    fn temp_account() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("synara-x509-{nanos}"));
        fs::create_dir_all(&dir).expect("temp account");
        dir
    }

    #[test]
    fn flag_defaults_off_and_does_not_inject() {
        let dir = temp_account();
        let runtime = load_runtime(&dir);
        assert!(!runtime.enabled);
        assert!(!runtime.has_ca());
        assert!(!runtime.should_inject_verifier());
        assert!(!should_inject_verifier(&dir));
        let status = status_from_store(&dir, Vec::new());
        assert!(!status.enabled);
        assert!(!status.verifier_configured);
        assert!(!status.reload_required);
        let json = serde_json::to_string(&status).expect("status json");
        assert!(!json_contains_secret_material(&json));
        assert!(!json.contains("BEGIN"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn enable_without_ca_is_a_noop_for_inject() {
        let dir = temp_account();
        set_enabled(&dir, true).expect("enable");
        assert!(load_runtime(&dir).enabled);
        assert!(!should_inject_verifier(&dir));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn import_ca_then_enable_injects() {
        let dir = temp_account();
        import_ca_pem(&dir, TEST_CA_PEM).expect("import ca");
        assert!(!should_inject_verifier(&dir));
        set_enabled(&dir, true).expect("enable");
        let runtime = load_runtime(&dir);
        assert!(runtime.has_ca());
        assert_eq!(runtime.cas.len(), 1);
        assert!(runtime.cas[0].fingerprint.starts_with("sha256:"));
        assert!(should_inject_verifier(&dir));
        record_applied(&dir, true);
        let status = status_from_store(&dir, vec!["@bot:example.org".into()]);
        assert!(status.verifier_configured);
        assert!(!status.reload_required);
        assert_eq!(
            status.certificate_verified_identities,
            vec!["@bot:example.org"]
        );
        let json = serde_json::to_string(&status).expect("status json");
        assert!(!json_contains_secret_material(&json));
        assert!(!json.contains("BEGIN CERTIFICATE"));
        assert!(!json.contains(TEST_CA_PEM.trim()));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn invalid_pem_is_rejected() {
        let dir = temp_account();
        assert_eq!(
            import_ca_pem(&dir, "not-a-certificate").unwrap_err(),
            X509StoreError::InvalidPem
        );
        assert!(!should_inject_verifier(&dir));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn remove_ca_by_fingerprint() {
        let dir = temp_account();
        import_ca_pem(&dir, TEST_CA_PEM).expect("import");
        set_enabled(&dir, true).expect("enable");
        let fingerprint = load_runtime(&dir).cas[0].fingerprint.clone();
        remove_ca(&dir, &fingerprint).expect("remove");
        assert!(!load_runtime(&dir).has_ca());
        assert!(!should_inject_verifier(&dir));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn reload_required_when_applied_lags_desired_inject() {
        let dir = temp_account();
        import_ca_pem(&dir, TEST_CA_PEM).expect("import");
        set_enabled(&dir, true).expect("enable");
        let status = status_from_store(&dir, Vec::new());
        assert!(status.reload_required);
        assert!(!status.verifier_configured);
        record_applied(&dir, true);
        let after = status_from_store(&dir, Vec::new());
        assert!(!after.reload_required);
        assert!(after.verifier_configured);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn leftover_olm_builder_source_has_no_x509_hooks() {
        let source = include_str!("../auth/crypto_device.rs");
        assert!(source.contains("OlmMachineBuilder::new"));
        assert!(!source.contains("with_x509"));
    }

    #[test]
    fn synara_x509_never_installs_ring_as_default_provider() {
        let source = include_str!("mod.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("production module precedes tests");
        let ring_install = ["ring", "::default_provider"].concat();
        assert!(!production.contains(&ring_install));
        assert!(!production.contains("crypto::ring"));
        assert!(production.contains("aws_lc_rs"));
        assert!(production.contains("should_inject_verifier"));
        let open = include_str!("../client_builder/open.rs");
        assert!(!open.contains(&ring_install));
        assert!(open.contains("ensure_aws_lc_rustls_provider"));
    }

    #[test]
    fn sas_owner_stays_sas_v1_only() {
        let live = include_str!("../verification/live.rs");
        assert!(live.contains("VerificationMethod::SasV1"));
        assert!(live.contains("is_self_verification"));
        assert!(!live.contains("with_x509"));
        assert!(!live.contains("io.element.x509"));
        // 1140 advertises QR *verification* (show + reciprocate) beside SAS.
        // X.509 must still not add a method. QR login stays out of this file.
        assert!(!live.contains("QrCodeScanV1"));
        let methods = live.matches("VerificationMethod::").count();
        let allowed = live.matches("VerificationMethod::SasV1").count()
            + live.matches("VerificationMethod::QrCodeShowV1").count()
            + live.matches("VerificationMethod::ReciprocateV1").count();
        assert_eq!(methods, allowed, "X.509 must not add a VerificationMethod");
    }

    #[test]
    fn master_key_json_detects_element_x509_algorithm() {
        assert!(master_key_has_x509_signature(
            r#"{"signatures":{"@a:b":{"io.element.x509:abcd":"sig"}}}"#
        ));
        assert!(!master_key_has_x509_signature(
            r#"{"signatures":{"@a:b":{"ed25519:DEVICE":"sig"}}}"#
        ));
    }

    #[cfg(feature = "x509-identity")]
    #[test]
    fn rust_verifier_constructs_from_imported_ca_without_ring_install() {
        ensure_aws_lc_rustls_provider();
        assert!(
            matrix_sdk_crypto::x509::RustRawX509Verifier::new_from_pem_data(TEST_CA_PEM).is_ok()
        );
        assert!(matrix_sdk_crypto::x509::RustRawX509Verifier::new_from_pem_data("nope").is_err());
        assert!(build_verifier(TEST_CA_PEM).is_some());
        assert!(build_verifier("nope").is_none());
        // rustls HTTPS builder must share the aws-lc process default (no panic).
        let _https = rustls::ClientConfig::builder();
        assert!(rustls::crypto::CryptoProvider::get_default().is_some());
    }
}
