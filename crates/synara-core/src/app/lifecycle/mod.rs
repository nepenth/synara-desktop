//! Native lifecycle primitives: verified account-store wipe and host vault
//! persist/restore/token rotation. Desktop owns Keyring I/O; Core owners call
//! these operations directly. Unused supervisor/task/remote-flow models are
//! retired; they do not define the product logout completion contract.

mod error;
mod session_material;
mod session_persist;
mod session_restore;
mod wipe;

pub use error::LifecycleError;
pub use session_material::{
    clear_session_material, load_session_material, persist_session_material,
    rotate_persisted_session_tokens, HostMatrixSessionSecrets, InMemorySessionMaterialVault,
    SessionMaterial, SessionMaterialId, SessionMaterialMeta, SessionMaterialVault,
    SESSION_ENVELOPE_VERSION, SESSION_KIND_MATRIX, SESSION_MATERIAL_SERVICE,
};
pub use session_persist::{
    persist_session_after_login, session_material_from_auth_session, SessionPersistOutcome,
};
pub use session_restore::{
    has_persisted_session, matrix_session_from_host_secrets, restore_session_from_vault,
    restore_session_from_vault_with_room_load_settings, restore_session_onto_client,
    restore_session_onto_client_with_room_load_settings, SessionRestoreOutcome,
};
pub use wipe::{
    assert_exact_account_root, assert_path_is_wipe_allowed, wipe_account_store, WipeReport,
    WipeTarget, WIPE_TARGET_KIND_ACCOUNT_ROOT,
};
