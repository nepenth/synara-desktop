//! Thin desktop adapters for the transport-neutral shared Core.

pub(crate) mod auth_probes;
pub(crate) mod cross_signing_status;
pub(crate) mod device_delete;
pub(crate) mod device_rename;
pub(crate) mod device_snapshot;
pub(crate) mod global_image_packs;
pub(crate) mod image_pack_writes;
pub(crate) mod join_rule_snapshot;
pub(crate) mod media_config;
pub(crate) mod presence_snapshot;
pub(crate) mod presence_subscriptions;
pub(crate) mod room_image_packs;
pub(crate) mod secret_storage_status;
pub(crate) mod session_lifecycle;
pub(crate) mod typing_set;
pub(crate) mod typing_snapshot;
pub(crate) mod user_image_pack;
pub(crate) mod verification_accept;
pub(crate) mod verification_begin_sas;
pub(crate) mod verification_cancel;
pub(crate) mod verification_confirm;
pub(crate) mod verification_dismiss;
pub(crate) mod verification_list;
pub(crate) mod verification_mismatch;
pub(crate) mod verification_start;
