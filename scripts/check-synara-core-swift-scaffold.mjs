#!/usr/bin/env node
/**
 * Host-neutral P4-1 contract check. It proves the committed inputs describe a
 * real, reproducible Apple build without pretending a non-Apple host produced
 * an XCFramework or generated Swift output.
 */
import { accessSync, readFileSync } from "node:fs";
import { constants } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(fileURLToPath(new URL("..", import.meta.url)));
const required = [
  "crates/synara-core/src/synara_core.udl",
  "crates/synara-core/src/ffi.rs",
  "crates/synara-core/src/session_projection_ffi.rs",
  "crates/synara-core/src/shared_core_ffi.rs",
  "crates/synara-core/src/platform/ios_fail_closed.rs",
  "crates/synara-core/build.rs",
  "crates/synara-core-bindgen/Cargo.toml",
  "crates/synara-core-bindgen/src/main.rs",
  "synara-ios/SynaraCore/Package.swift",
  "synara-ios/SynaraCore/Sources/SynaraCore/SynaraCore.swift",
  "synara-ios/Synara/Services/MatrixSessionProjectionMirror.swift",
  "synara-ios/Synara/Services/IosSecretVault.swift",
  "synara-ios/Synara/Services/SharedCoreSessionRestore.swift",
  "synara-ios/Synara/Services/SharedCoreSessionLogin.swift",
  "synara-ios/Synara/Services/SharedCoreSessionAttach.swift",
  "synara-ios/Synara/Services/SharedCoreRoomList.swift",
  "synara-ios/Synara/Services/SharedCoreInvites.swift",
  "synara-ios/Synara/Services/SharedCoreTimeline.swift",
  "synara-ios/Synara/Services/SharedCoreTypingPresence.swift",
  "synara-ios/Synara/Services/SharedCoreVerificationList.swift",
  "synara-ios/Synara/Services/SharedCoreVerificationSas.swift",
  "synara-ios/Synara/Services/SharedCoreDevices.swift",
  "synara-ios/SynaraTests/SynaraCoreBindingsTests.swift",
  "synara-ios/SynaraCore/Sources/synara_coreFFI/include/.gitkeep",
  "synara-ios/SynaraCore/.gitignore",
  "scripts/generate-synara-core-swift.sh",
  "synara-ios/scripts/ci-build.sh",
  ".github/workflows/ci.yml",
];
for (const path of required) {
  try {
    accessSync(resolve(root, path), constants.R_OK);
  } catch {
    throw new Error(`missing P4-1 scaffold input: ${path}`);
  }
}

const cargo = readFileSync(resolve(root, "crates/synara-core/Cargo.toml"), "utf8");
const udl = readFileSync(resolve(root, "crates/synara-core/src/synara_core.udl"), "utf8");
const lib = readFileSync(resolve(root, "crates/synara-core/src/lib.rs"), "utf8");
const ffi = readFileSync(resolve(root, "crates/synara-core/src/ffi.rs"), "utf8");
const sessionProjectionFfi = readFileSync(
  resolve(root, "crates/synara-core/src/session_projection_ffi.rs"),
  "utf8"
);
const sharedCoreFfi = readFileSync(
  resolve(root, "crates/synara-core/src/shared_core_ffi.rs"),
  "utf8"
);
const sessionProjectionAdapter = readFileSync(
  resolve(root, "synara-ios/Synara/Services/MatrixSessionProjectionMirror.swift"),
  "utf8"
);
const sharedCoreRestore = readFileSync(
  resolve(root, "synara-ios/Synara/Services/SharedCoreSessionRestore.swift"),
  "utf8"
);
const sharedCoreLogin = readFileSync(
  resolve(root, "synara-ios/Synara/Services/SharedCoreSessionLogin.swift"),
  "utf8"
);
const sharedCoreAttach = readFileSync(
  resolve(root, "synara-ios/Synara/Services/SharedCoreSessionAttach.swift"),
  "utf8"
);
const sharedCoreRoomList = readFileSync(
  resolve(root, "synara-ios/Synara/Services/SharedCoreRoomList.swift"),
  "utf8"
);
const sharedCoreInvites = readFileSync(
  resolve(root, "synara-ios/Synara/Services/SharedCoreInvites.swift"),
  "utf8"
);
const sharedCoreTimeline = readFileSync(
  resolve(root, "synara-ios/Synara/Services/SharedCoreTimeline.swift"),
  "utf8"
);
const sharedCoreTypingPresence = readFileSync(
  resolve(root, "synara-ios/Synara/Services/SharedCoreTypingPresence.swift"),
  "utf8"
);
const sharedCoreVerificationList = readFileSync(
  resolve(root, "synara-ios/Synara/Services/SharedCoreVerificationList.swift"),
  "utf8"
);
const sharedCoreVerificationSas = readFileSync(
  resolve(root, "synara-ios/Synara/Services/SharedCoreVerificationSas.swift"),
  "utf8"
);
const sharedCoreDevices = readFileSync(
  resolve(root, "synara-ios/Synara/Services/SharedCoreDevices.swift"),
  "utf8"
);
const swiftBindingsTests = readFileSync(
  resolve(root, "synara-ios/SynaraTests/SynaraCoreBindingsTests.swift"),
  "utf8"
);
const matrixRustSDKService = readFileSync(
  resolve(root, "synara-ios/Synara/Services/MatrixRustSDKService.swift"),
  "utf8"
);
const appServices = readFileSync(resolve(root, "synara-ios/Synara/Services/AppServices.swift"), "utf8");
const settingsView = readFileSync(resolve(root, "synara-ios/Synara/Features/SettingsView.swift"), "utf8");
const packageManifest = readFileSync(resolve(root, "synara-ios/SynaraCore/Package.swift"), "utf8");
const ignored = readFileSync(resolve(root, "synara-ios/SynaraCore/.gitignore"), "utf8");
const generator = readFileSync(resolve(root, "scripts/generate-synara-core-swift.sh"), "utf8");
const iosCiBuild = readFileSync(resolve(root, "synara-ios/scripts/ci-build.sh"), "utf8");
const ciWorkflow = readFileSync(resolve(root, ".github/workflows/ci.yml"), "utf8");

const assertions = [
  [cargo, 'crate-type = ["lib", "staticlib", "cdylib"]', "Apple library crate types"],
  [cargo, 'uniffi = { version = "=0.28.3" }', "pinned UniFFI runtime"],
  [readFileSync(resolve(root, "crates/synara-core-bindgen/Cargo.toml"), "utf8"), 'uniffi = { version = "=0.28.3", features = ["cli"] }', "pinned project-owned UniFFI generator"],
  [cargo, 'features = ["build"]', "UniFFI build scaffolding"],
  [udl, "namespace synara_core", "project-owned UniFFI namespace"],
  [udl, "binding_scaffold_version", "P4-1 binding bootstrap"],
  [udl, "[Async, Throws=LoginFlowsError]", "async typed login-flow operation"],
  [udl, "sequence<LoginFlowDto> login_flows(string homeserver_url)", "typed login-flow return"],
  [udl, "dictionary LoginFlowDto", "typed login-flow DTO"],
  [udl, "boolean? get_login_token", "optional token-capability metadata"],
  [udl, "interface LoginFlowsError", "typed privacy-safe login-flow error"],
  [udl, "RegisterFlowsDto register_flows(string homeserver_url)", "typed registration-flow operation"],
  [udl, "dictionary RegisterFlowsDto", "closed registration-flow DTO"],
  [udl, "interface RegisterFlowsError", "typed privacy-safe registration-flow error"],
  [udl, "dictionary SessionProjection", "P4-3 safe session-projection record"],
  [udl, "interface SessionProjectionCore", "P4-3 project-owned session facade"],
  [udl, "SessionProjection? session_snapshot()", "P4-3 projection snapshot operation"],
  [udl, "interface SessionProjectionError", "P4-3 static privacy-safe error"],
  [udl, "interface SharedCore", "P4-S2 construction-only shared Core facade"],
  [lib, 'uniffi::include_scaffolding!("synara_core")', "Rust FFI scaffolding inclusion"],
  [lib, "SessionProjectionCore", "P4-3 facade export"],
  [lib, "SharedCore", "P4-S2 shared Core facade export"],
  [sessionProjectionFfi, "Core::with_registry", "P4-3 Core open/close/snapshot delegation"],
  [sessionProjectionFfi, "CommandRegistry::new()", "P4-3 facade has no command registry"],
  [sessionProjectionFfi, "uniffi_projection_facade_executes_core_open_snapshot_and_close", "P4-3 Rust behavioral facade test"],
  [sessionProjectionFfi, "facade_rejects_hostile_values_with_static_privacy_safe_error", "P4-3 Rust hostile-input privacy test"],
  [sharedCoreFfi, "SharedCore", "P4-S2 shared Core facade"],
  [sharedCoreFfi, "IosFailClosedPlatform", "P4-S2 Rust-owned iOS Platform"],
  [sharedCoreFfi, "Core::new", "P4-S2 real Core construction"],
  [sharedCoreFfi, "new_with_secret_store", "P4-S3a vault-backed SharedCore constructor"],
  [sharedCoreFfi, "CallbackSecretVault", "P4-S3a callback SecretVault adapter"],
  [sharedCoreFfi, "pub trait IosSecretVault", "P4-S3a callback trait defined in Rust"],
  [sharedCoreFfi, "restore_persisted_session", "P4-S3b vault restore FFI"],
  [sharedCoreFfi, "login_with_password", "P4-S3c dedicated password-login FFI"],
  [sharedCoreFfi, "persist_session_after_login", "P4-S3c persists into the S3a vault"],
  [sharedCoreFfi, "persist_planted_session_for_test", "P4-S3c test hook uses the production persist path"],
  [sharedCoreFfi, "persist_open_and_retain", "P4-S3c login and test hook share persist+open+retain"],
  [sharedCoreFfi, "Zeroizing::new(password)", "P4-S3c zeroizes the dedicated password argument"],
  [udl, "callback interface IosSecretVault", "P4-S3a Swift vault callback"],
  [udl, "interface IosSecretVaultError", "P4-S3a static vault error"],
  [udl, "restore_persisted_session", "P4-S3b SharedCore restore operation"],
  [udl, "dictionary SessionRestoreDto", "P4-S3b privacy-safe restore DTO"],
  [udl, "interface SessionRestoreError", "P4-S3b static restore error"],
  [udl, "login_with_password", "P4-S3c SharedCore dedicated login operation"],
  [udl, "dictionary SessionLoginDto", "P4-S3c privacy-safe login DTO"],
  [udl, "interface SessionLoginError", "P4-S3c static login error"],
  [sessionProjectionAdapter, "openAfterInstalledClient", "post-install projection hook"],
  [sessionProjectionAdapter, "closeBeforeSDKWipe", "pre-wipe projection close hook"],
  [sessionProjectionAdapter, "func coreSessionIdentity() async -> CoreSessionIdentity?", "P4-4 display-only Core identity readback"],
  [sessionProjectionAdapter, "try await core.sessionSnapshot()", "P4-4 mirror sessionSnapshot readback"],
  [sessionProjectionAdapter, "snapshot.lifecycle == .ready", "P4-4 ready-only Core identity readback"],
  [sessionProjectionAdapter, "identity == expectedIdentity, self.expectedIdentity == expectedIdentity", "P4-4 exact and concurrent-safe identity match"],
  [sessionProjectionAdapter, "expectedIdentity = nil\n        try? await core.close()", "P4-4 clears expected identity before awaiting Core close"],
  [appServices, "func coreSessionIdentity() async -> CoreSessionIdentity?", "P4-4 Matrix client display-only identity protocol"],
  [appServices, "extension MatrixClientServicing", "P4-4 Matrix client identity default"],
  [matrixRustSDKService, "await sessionProjectionMirror.coreSessionIdentity()", "P4-4 client store mirror-only identity readback"],
  [matrixRustSDKService, "await clientStore.coreSessionIdentity()", "P4-4 Matrix client service mirror-only identity readback"],
  [settingsView, "await refreshCoreSessionIdentity()", "P4-4 Settings task identity refresh"],
  [settingsView, "SettingsAccountIdentitySelection.matchingCoreIdentity", "P4-4 fail-closed Settings identity selection"],
  [swiftBindingsTests, "testSessionProjectionFacadeExecutesOpenSnapshotAndCloseOverGeneratedRustFFI", "Swift behavioral FFI test"],
  [swiftBindingsTests, "try await core.open", "Swift generated FFI open execution"],
  [swiftBindingsTests, "try await core.sessionSnapshot()", "Swift generated FFI snapshot execution"],
  [swiftBindingsTests, "try await core.close()", "Swift generated FFI close execution"],
  [swiftBindingsTests, "testSharedCoreConstructsOverGeneratedRustFFI", "Swift P4-S2 Core construction test"],
  [swiftBindingsTests, "testSharedCoreAcceptsInMemorySecretStore", "Swift P4-S3a vault constructor test"],
  [swiftBindingsTests, "testSharedCoreRestoreWithoutVaultFailsClosed", "Swift P4-S3b fail-closed restore test"],
  [swiftBindingsTests, "testSharedCoreRestoreRejectsHostileIdentityWithoutEcho", "Swift P4-S3b hostile-identity restore test"],
  [swiftBindingsTests, "testSharedCoreRestoreHoldsInstanceAcrossCalls", "Swift P4-S3b helper keeps caller-owned SharedCore"],
  [sharedCoreRestore, "restorePersistedSession", "P4-S3b product restore helper"],
  [sharedCoreRestore, "core: SharedCore", "P4-S3b helper takes an already-constructed SharedCore"],
  [sharedCoreRestore, "core.restorePersistedSession", "P4-S3b helper restores on the caller-owned instance"],
  [swiftBindingsTests, "testSharedCoreLoginWithoutVaultFailsClosed", "Swift P4-S3c fail-closed login test"],
  [swiftBindingsTests, "testSharedCoreLoginRejectsHostileIdentityWithoutEchoingPassword", "Swift P4-S3c hostile-identity login test"],
  [sharedCoreLogin, "loginWithPassword", "P4-S3c product login helper"],
  [sharedCoreLogin, "core: SharedCore", "P4-S3c helper takes an already-constructed SharedCore"],
  [sharedCoreLogin, "core.loginWithPassword", "P4-S3c helper logs in on the caller-owned instance"],
  [sharedCoreFfi, "attach_session_owners", "P4-S3d attach FFI"],
  [sharedCoreFfi, "attach_typing", "P4-S3d wires Core attach_typing"],
  [sharedCoreFfi, "attach_presence", "P4-S3d wires Core attach_presence"],
  [sharedCoreFfi, "attach_verification", "P4-S3d wires Core attach_verification"],
  [sharedCoreFfi, "attach_devices", "P4-S3d wires Core attach_devices"],
  [sharedCoreFfi, "attach_join_rules", "P4-S3d wires Core attach_join_rules"],
  [sharedCoreFfi, "attach_image_packs", "P4-S3d wires Core attach_image_packs"],
  [sharedCoreFfi, "attach_timelines", "P4-S3d wires Core attach_timelines"],
  [sharedCoreFfi, "attach_sync", "P4-S3d wires Core attach_sync"],
  [udl, "attach_session_owners", "P4-S3d SharedCore attach operation"],
  [udl, "dictionary SessionAttachDto", "P4-S3d privacy-safe attach DTO"],
  [udl, "interface SessionAttachError", "P4-S3d static attach error"],
  [swiftBindingsTests, "testSharedCoreAttachWithoutSessionFailsClosed", "Swift P4-S3d fail-closed attach test"],
  [sharedCoreAttach, "attachSessionOwners", "P4-S3d product attach helper"],
  [sharedCoreAttach, "core: SharedCore", "P4-S3d helper takes an already-constructed SharedCore"],
  [sharedCoreAttach, "core.attachSessionOwners", "P4-S3d helper attaches on the caller-owned instance"],
  [sharedCoreFfi, "room_list_snapshot", "P4-S4 typed room-list FFI"],
  [sharedCoreFfi, "matrix_room_list_snapshot", "P4-S4 calls the registered Core command"],
  [sharedCoreFfi, "CommandEnvelope", "P4-S4 uses Core.command internally"],
  [udl, "RoomListSnapshotDto room_list_snapshot()", "P4-S4 SharedCore room-list operation"],
  [udl, "dictionary RoomListSnapshotDto", "P4-S4 privacy-safe room-list DTO"],
  [udl, "interface RoomListSnapshotError", "P4-S4 static room-list error"],
  [swiftBindingsTests, "testSharedCoreRoomListWithoutSessionFailsClosed", "Swift P4-S4 fail-closed room-list test"],
  [sharedCoreRoomList, "roomListSnapshot", "P4-S4 product room-list helper"],
  [sharedCoreRoomList, "core: SharedCore", "P4-S4 helper takes an already-constructed SharedCore"],
  [sharedCoreRoomList, "core.roomListSnapshot", "P4-S4 helper reads on the caller-owned instance"],
  [sharedCoreFfi, "invites_snapshot", "P4-S5 typed invite FFI"],
  [sharedCoreFfi, "matrix_invites_snapshot", "P4-S5 calls the registered Core command"],
  [udl, "InviteSnapshotDto invites_snapshot()", "P4-S5 SharedCore invite operation"],
  [udl, "dictionary InviteSnapshotDto", "P4-S5 privacy-safe invite DTO"],
  [udl, "interface InviteSnapshotError", "P4-S5 static invite error"],
  [swiftBindingsTests, "testSharedCoreInvitesWithoutSessionFailsClosed", "Swift P4-S5 fail-closed invite test"],
  [sharedCoreInvites, "invitesSnapshot", "P4-S5 product invite helper"],
  [sharedCoreInvites, "core: SharedCore", "P4-S5 helper takes an already-constructed SharedCore"],
  [sharedCoreInvites, "core.invitesSnapshot", "P4-S5 helper reads on the caller-owned instance"],
  [sharedCoreFfi, "timeline_open", "P4-S6 typed timeline-open FFI"],
  [sharedCoreFfi, "matrix_timeline_open", "P4-S6 calls the registered open command"],
  [sharedCoreFfi, "matrix_timeline_close", "P4-S6 calls the registered close command"],
  [sharedCoreFfi, "matrix_timeline_paginate", "P4-S6 calls the registered paginate command"],
  [udl, "TimelineOpenDto timeline_open(", "P4-S6 SharedCore timeline-open operation"],
  [udl, "boolean timeline_close(", "P4-S6 SharedCore timeline-close operation"],
  [udl, "TimelineSnapshotDto timeline_paginate(", "P4-S6 SharedCore timeline-paginate operation"],
  [udl, "dictionary TimelineOpenDto", "P4-S6 privacy-safe timeline-open DTO"],
  [udl, "interface TimelineError", "P4-S6 static timeline error"],
  [swiftBindingsTests, "testSharedCoreTimelineWithoutSessionFailsClosed", "Swift P4-S6 fail-closed timeline test"],
  [sharedCoreTimeline, "timelineOpen", "P4-S6 product timeline-open helper"],
  [sharedCoreTimeline, "timelineClose", "P4-S6 product timeline-close helper"],
  [sharedCoreTimeline, "timelinePaginate", "P4-S6 product timeline-paginate helper"],
  [sharedCoreTimeline, "core: SharedCore", "P4-S6 helper takes an already-constructed SharedCore"],
  [sharedCoreTimeline, "core.timelineOpen", "P4-S6 helper opens on the caller-owned instance"],
  [sharedCoreFfi, "typing_snapshot", "P4-S7 typed typing-snapshot FFI"],
  [sharedCoreFfi, "matrix_typing_snapshot", "P4-S7 calls the registered typing snapshot"],
  [sharedCoreFfi, "matrix_typing_set", "P4-S7 calls the registered typing set"],
  [sharedCoreFfi, "matrix_presence_snapshot", "P4-S7 calls the registered presence snapshot"],
  [sharedCoreFfi, "matrix_presence_subscribe", "P4-S7 calls the registered presence subscribe"],
  [sharedCoreFfi, "matrix_presence_unsubscribe", "P4-S7 calls the registered presence unsubscribe"],
  [udl, "TypingSnapshotDto typing_snapshot()", "P4-S7 SharedCore typing-snapshot operation"],
  [udl, "void typing_set(", "P4-S7 SharedCore typing-set operation"],
  [udl, "PresenceSnapshotDto presence_snapshot(", "P4-S7 SharedCore presence-snapshot operation"],
  [udl, "PresenceSubscriptionDto presence_subscribe(", "P4-S7 SharedCore presence-subscribe operation"],
  [udl, "void presence_unsubscribe(", "P4-S7 SharedCore presence-unsubscribe operation"],
  [udl, "interface TypingCommandError", "P4-S7 static typing error"],
  [udl, "interface PresenceCommandError", "P4-S7 static presence error"],
  [swiftBindingsTests, "testSharedCoreTypingPresenceWithoutSessionFailsClosed", "Swift P4-S7 fail-closed typing/presence test"],
  [sharedCoreTypingPresence, "typingSnapshot", "P4-S7 product typing-snapshot helper"],
  [sharedCoreTypingPresence, "presenceSubscribe", "P4-S7 product presence-subscribe helper"],
  [sharedCoreTypingPresence, "core: SharedCore", "P4-S7 helper takes an already-constructed SharedCore"],
  [sharedCoreTypingPresence, "core.typingSnapshot", "P4-S7 helper reads on the caller-owned instance"],
  [sharedCoreFfi, "verification_list", "P4-S8 typed verification-list FFI"],
  [sharedCoreFfi, "matrix_verification_list", "P4-S8 calls the registered Core command"],
  [udl, "VerificationInboxDto verification_list()", "P4-S8 SharedCore verification-list operation"],
  [udl, "dictionary VerificationInboxDto", "P4-S8 privacy-safe verification inbox DTO"],
  [udl, "interface VerificationListError", "P4-S8 static verification-list error"],
  [swiftBindingsTests, "testSharedCoreVerificationListWithoutSessionFailsClosed", "Swift P4-S8 fail-closed verification-list test"],
  [sharedCoreVerificationList, "verificationList", "P4-S8 product verification-list helper"],
  [sharedCoreVerificationList, "core: SharedCore", "P4-S8 helper takes an already-constructed SharedCore"],
  [sharedCoreVerificationList, "core.verificationList", "P4-S8 helper reads on the caller-owned instance"],
  [sharedCoreFfi, "verification_start", "P4-S9 typed verification-start FFI"],
  [sharedCoreFfi, "matrix_verification_start", "P4-S9 calls the registered start command"],
  [sharedCoreFfi, "matrix_verification_accept", "P4-S9 calls the registered accept command"],
  [sharedCoreFfi, "matrix_verification_begin_sas", "P4-S9 calls the registered begin_sas command"],
  [sharedCoreFfi, "matrix_verification_confirm", "P4-S9 calls the registered confirm command"],
  [sharedCoreFfi, "matrix_verification_mismatch", "P4-S9 calls the registered mismatch command"],
  [sharedCoreFfi, "matrix_verification_cancel", "P4-S9 calls the registered cancel command"],
  [sharedCoreFfi, "matrix_verification_dismiss", "P4-S9 calls the registered dismiss command"],
  [udl, "VerificationRequestDto verification_start(", "P4-S9 SharedCore verification-start operation"],
  [udl, "VerificationRequestDto verification_accept(", "P4-S9 SharedCore verification-accept operation"],
  [udl, "VerificationRequestDto verification_begin_sas(", "P4-S9 SharedCore verification-begin-sas operation"],
  [udl, "VerificationRequestDto verification_confirm(", "P4-S9 SharedCore verification-confirm operation"],
  [udl, "VerificationRequestDto verification_mismatch(", "P4-S9 SharedCore verification-mismatch operation"],
  [udl, "VerificationRequestDto verification_cancel(", "P4-S9 SharedCore verification-cancel operation"],
  [udl, "void verification_dismiss(", "P4-S9 SharedCore verification-dismiss operation"],
  [udl, "dictionary VerificationSasDto", "P4-S9 privacy-safe SAS DTO"],
  [udl, "interface VerificationSasError", "P4-S9 static verification-SAS error"],
  [swiftBindingsTests, "testSharedCoreVerificationSasWithoutSessionFailsClosed", "Swift P4-S9 fail-closed verification-SAS test"],
  [sharedCoreVerificationSas, "verificationStart", "P4-S9 product verification-start helper"],
  [sharedCoreVerificationSas, "verificationBeginSas", "P4-S9 product begin-sas helper"],
  [sharedCoreVerificationSas, "verificationDismiss", "P4-S9 product dismiss helper"],
  [sharedCoreVerificationSas, "core: SharedCore", "P4-S9 helper takes an already-constructed SharedCore"],
  [sharedCoreVerificationSas, "core.verificationStart", "P4-S9 helper starts on the caller-owned instance"],
  [sharedCoreFfi, "device_snapshot", "P4-S9-2 typed device-snapshot FFI"],
  [sharedCoreFfi, "matrix_device_snapshot", "P4-S9-2 calls the registered snapshot command"],
  [sharedCoreFfi, "matrix_device_rename", "P4-S9-2 calls the registered rename command"],
  [sharedCoreFfi, "matrix_device_delete_start", "P4-S9-2 calls the registered delete-start command"],
  [sharedCoreFfi, "matrix_device_delete_cancel", "P4-S9-2 calls the registered delete-cancel command"],
  [sharedCoreFfi, "matrix_backup_status", "P4-S9-2 calls the registered backup-status command"],
  [sharedCoreFfi, "matrix_room_key_transfer_status", "P4-S9-2 calls the registered room-key status command"],
  [sharedCoreFfi, "matrix_cross_signing_setup", "P4-S9-2 calls the registered cross-signing setup command"],
  [udl, "DeviceSnapshotDto device_snapshot()", "P4-S9-2 SharedCore device-snapshot operation"],
  [udl, "DeviceSnapshotDto device_rename(", "P4-S9-2 SharedCore device-rename operation"],
  [udl, "DeviceDeleteDto device_delete_start(", "P4-S9-2 SharedCore delete-start operation"],
  [udl, "void device_delete_cancel(", "P4-S9-2 SharedCore delete-cancel operation"],
  [udl, "BackupStatusDto backup_status()", "P4-S9-2 SharedCore backup-status operation"],
  [udl, "RoomKeyTransferStatusDto room_key_transfer_status()", "P4-S9-2 SharedCore room-key status operation"],
  [udl, "CrossSigningSetupDto cross_signing_setup()", "P4-S9-2 SharedCore cross-signing setup operation"],
  [udl, "interface DeviceCommandError", "P4-S9-2 static device-family error"],
  [swiftBindingsTests, "testSharedCoreDevicesWithoutSessionFailsClosed", "Swift P4-S9-2 fail-closed device-family test"],
  [sharedCoreDevices, "deviceSnapshot", "P4-S9-2 product device-snapshot helper"],
  [sharedCoreDevices, "deviceRename", "P4-S9-2 product rename helper"],
  [sharedCoreDevices, "deviceDeleteStart", "P4-S9-2 product delete-start helper"],
  [sharedCoreDevices, "deviceDeleteCancel", "P4-S9-2 product delete-cancel helper"],
  [sharedCoreDevices, "backupStatus", "P4-S9-2 product backup-status helper"],
  [sharedCoreDevices, "roomKeyTransferStatus", "P4-S9-2 product room-key status helper"],
  [sharedCoreDevices, "crossSigningSetup", "P4-S9-2 product cross-signing setup helper"],
  [sharedCoreDevices, "core: SharedCore", "P4-S9-2 helper takes an already-constructed SharedCore"],
  [sharedCoreDevices, "core.deviceSnapshot", "P4-S9-2 helper reads on the caller-owned instance"],
  [swiftBindingsTests, "testProductionMirrorReadsReadyCoreIdentityThenClearsOnClose", "P4-4 production mirror readback test"],
  [swiftBindingsTests, "testMirrorFailsClosedForMismatchedNonReadyAndMissingCoreSnapshots", "P4-4 mirror mismatch/nil fallback test"],
  [swiftBindingsTests, "testMirrorDoesNotPublishAnIdentityWhenCoreOpenFails", "P4-4 failed Core open fallback test"],
  [
    matrixRustSDKService,
    "self.client = client\n            activeSession = session\n            await sessionProjectionMirror.openAfterInstalledClient",
    "login mirror only after MatrixRustSDK client install",
  ],
  [
    matrixRustSDKService,
    "client = newClient\n            activeSession = session\n            await sessionProjectionMirror.openAfterInstalledClient",
    "restore mirror only after MatrixRustSDK client install",
  ],
  [
    matrixRustSDKService,
    "await sessionProjectionMirror.closeBeforeSDKWipe()\n        retainClientHandle(client)",
    "projection close before SDK client release/wipe",
  ],
  [ffi, "discover_login_flows", "shared-core login-flow discovery call"],
  [ffi, "HttpLoginFlowTransport::new()", "bounded Core login-flow transport"],
  [ffi, "probe_register_flows", "shared-core registration-flow probe call"],
  [ffi, "HttpRegisterFlowTransport::new()", "bounded Core registration-flow transport"],
  [readFileSync(resolve(root, "crates/synara-core/build.rs"), "utf8"), "unexpected UniFFI 0.28.3 metadata-doc shape", "fail-closed generated-scaffolding lint patch"],
  [packageManifest, 'name: "SynaraCore"', "Swift package target"],
  [packageManifest, 'name: "synara_coreFFI"', "generated C FFI binary target"],
  [packageManifest, '.binaryTarget(', "generated XCFramework binary target"],
  [packageManifest, 'path: "Artifacts/SynaraCore.xcframework"', "generated XCFramework path"],
  [packageManifest, 'dependencies: ["synara_coreFFI"]', "Swift-to-generated-FFI dependency"],
  [packageManifest, 'path: "Sources/SynaraCore"', "generated-source package target"],
  [ignored, "/Sources/SynaraCore/Generated/*.swift", "generated Swift exclusion"],
  [ignored, "/Sources/synara_coreFFI/include/*", "generated C FFI exclusion"],
  [ignored, "/Artifacts/", "generated XCFramework exclusion"],
  [ignored, "/.build/", "Swift package build-product exclusion"],
  [generator, '[[ "$(uname -s)" != "Darwin" ]]', "clear non-Apple failure"],
  [generator, "aarch64-apple-ios-sim", "Apple Silicon simulator target"],
  [generator, "x86_64-apple-ios", "Intel simulator target"],
  [generator, "xcrun lipo -create", "combined generic simulator library"],
  [generator, "aarch64-apple-darwin", "Apple macOS target"],
  [generator, "cargo build --locked --release --package synara-core", "locked Rust build"],
  [generator, "cargo run --locked --package synara-core-bindgen", "project-owned locked bindgen invocation"],
  [generator, "-- generate", "project-owned bindgen command"],
  [generator, "--no-format", "source-preserving bindgen invocation"],
  [generator, 'synara_coreFFI.modulemap', "generated FFI module map publication"],
  [generator, 'module.modulemap', "XCFramework module map structure"],
  [generator, '-headers "$headers_tmp"', "XCFramework generated C header inclusion"],
  [generator, "xcodebuild -create-xcframework", "XCFramework assembly"],
];
for (const [text, needle, label] of assertions) {
  if (!text.includes(needle)) throw new Error(`missing ${label}: ${needle}`);
}

// The app's MatrixRustSDK service constructs this actor from another file in
// the same app target, so it must be module-internal. Parse enough Swift
// lexically to remove comments and string literals before inspecting the one
// declaration. The scan binds access modifiers across arbitrary whitespace:
// a modifier split onto a preceding line must not evade the privacy contract.
const stripSwiftCommentsAndStrings = (source) => {
  let output = "";
  let index = 0;
  const blank = (text) => text.replace(/[^\n]/g, " ");
  while (index < source.length) {
    if (source.startsWith("//", index)) {
      const end = source.indexOf("\n", index);
      const stop = end === -1 ? source.length : end;
      output += blank(source.slice(index, stop));
      index = stop;
    } else if (source.startsWith("/*", index)) {
      let depth = 1;
      let cursor = index + 2;
      while (cursor < source.length && depth > 0) {
        if (source.startsWith("/*", cursor)) {
          depth += 1;
          cursor += 2;
        } else if (source.startsWith("*/", cursor)) {
          depth -= 1;
          cursor += 2;
        } else {
          cursor += 1;
        }
      }
      output += blank(source.slice(index, cursor));
      index = cursor;
    } else if (source.startsWith('"""', index)) {
      let cursor = index + 3;
      while (cursor < source.length && !source.startsWith('"""', cursor)) cursor += 1;
      cursor = Math.min(source.length, cursor + 3);
      output += blank(source.slice(index, cursor));
      index = cursor;
    } else if (source[index] === '"') {
      let cursor = index + 1;
      while (cursor < source.length) {
        if (source[cursor] === "\\") cursor += 2;
        else if (source[cursor++] === '"') break;
      }
      output += blank(source.slice(index, cursor));
      index = cursor;
    } else {
      output += source[index++];
    }
  }
  return output;
};

const swiftTokens = (source) => [
  ...source.matchAll(/[A-Za-z_][A-Za-z0-9_]*|[{}:]/g),
].map((match) => ({ value: match[0], index: match.index }));

const accessModifiers = new Set([
  "public",
  "open",
  "private",
  "fileprivate",
  "package",
  "internal",
  "final",
  "nonisolated",
  "isolated",
  "static",
  "class",
  "distributed",
]);

const isModuleInternalProjectionActor = (source) => {
  const stripped = stripSwiftCommentsAndStrings(source);
  // Conditional branches make a lightweight source scanner ambiguous. This
  // one-file internal adapter has no need for them, so reject rather than
  // guessing which declaration Xcode activates.
  if (/^\s*#\s*(?:if|elseif|else|endif)\b/m.test(stripped) || stripped.includes("@")) {
    return false;
  }

  const tokens = swiftTokens(stripped);
  const namePositions = tokens
    .map((token, index) => (token.value === "MatrixSessionProjectionMirror" ? index : -1))
    .filter((index) => index >= 0);
  if (namePositions.length !== 1) return false;

  const name = namePositions[0];
  if (tokens[name - 1]?.value !== "actor") return false;
  if (!new Set(["{", ":"]).has(tokens[name + 1]?.value)) return false;

  // The adapter must be a file-scope declaration. An unmodified nested actor
  // inherits an enclosing type/extension's access, so it is not enough to
  // inspect only the actor's direct prefix. Track braces from the token stream
  // and reject any declaration inside a type or extension body.
  let braceDepth = 0;
  for (const token of tokens.slice(0, name)) {
    if (token.value === "{") braceDepth += 1;
    if (token.value === "}") {
      braceDepth -= 1;
      if (braceDepth < 0) return false;
    }
  }
  if (braceDepth !== 0) return false;

  // Bind every consecutive declaration modifier before `actor` irrespective
  // of newlines/comments. Only no modifier or the explicit `internal` marker
  // is accepted. A preceding import/other declaration is not a modifier and
  // therefore correctly terminates this declaration-prefix scan.
  const modifiers = [];
  for (let index = name - 2; index >= 0 && accessModifiers.has(tokens[index].value); index -= 1) {
    modifiers.unshift(tokens[index].value);
  }
  return modifiers.length === 0 || (modifiers.length === 1 && modifiers[0] === "internal");
};

for (const [fixture, expected] of [
  ["actor MatrixSessionProjectionMirror {}", true],
  ["internal actor MatrixSessionProjectionMirror {}", true],
  ["internal\nactor MatrixSessionProjectionMirror {}", true],
  ["public actor MatrixSessionProjectionMirror {}", false],
  ["public\nactor MatrixSessionProjectionMirror {}", false],
  ["private\n/* comment */\nactor MatrixSessionProjectionMirror {}", false],
  ["// actor MatrixSessionProjectionMirror {}\npublic actor Other {}", false],
  ['let bait = "actor MatrixSessionProjectionMirror {}"', false],
  ["fileprivate actor MatrixSessionProjectionMirror {}", false],
  ["package actor MatrixSessionProjectionMirror {}", false],
  ["package\nactor MatrixSessionProjectionMirror {}", false],
  ["distributed actor MatrixSessionProjectionMirror {}", false],
  ["public distributed actor MatrixSessionProjectionMirror {}", false],
  ["private\ndistributed actor MatrixSessionProjectionMirror {}", false],
  ["final internal actor MatrixSessionProjectionMirror {}", false],
  ["final class MatrixSessionProjectionMirror {}", false],
  ["public extension PublicHost { actor MatrixSessionProjectionMirror {} }", false],
  ["struct Host {\ninternal actor MatrixSessionProjectionMirror {}\n}", false],
  ["#if false\nactor MatrixSessionProjectionMirror {}\n#endif", false],
  ["#if false\nactor MatrixSessionProjectionMirror {}\n#endif\npublic actor MatrixSessionProjectionMirror {}", false],
]) {
  if (isModuleInternalProjectionActor(fixture) !== expected) {
    throw new Error("P4-3 projection actor access parser fixture failed");
  }
}
if (!isModuleInternalProjectionActor(sessionProjectionAdapter)) {
  throw new Error("P4-3 projection adapter must be exactly one module-internal actor declaration");
}

// P4-4 intentionally reduces the generated snapshot to three display-only
// values before it reaches the app protocol. This tiny mirror has no aliases
// or extensions, so reject either file-wide rather than attempting Swift name
// resolution or extension ownership parsing.
const swiftStructuralTokens = (source) => [
  ...source.matchAll(/[A-Za-z_][A-Za-z0-9_]*|->|[{}()[\]<>:,.?!=@;]/g),
].map((match) => ({ value: match[0], index: match.index }));
const hasForbiddenProjectionMirrorDeclaration = (stripped) =>
  /\b(?:typealias|extension)\b/.test(stripped);
const approvedCoreIdentityFields = ["userID:String", "deviceID:String", "homeserverURL:String"];
const canonicalCoreIdentityHeader = "struct CoreSessionIdentity: Equatable, Sendable {";
const coreIdentityDeclarationModifier =
  /\b(?:public|open|private|fileprivate|package|internal|final|nonisolated|isolated|static|class|distributed)\s+struct\s+CoreSessionIdentity\b/;

// Return the exact field census only when the sole declaration is a file-scope
// struct with the literal unprefixed header and three stored fields. The
// modifier scan deliberately applies to the whole stripped file: a same-line
// preceding declaration must never hide a public (or otherwise prefixed)
// CoreSessionIdentity declaration.
const coreSessionIdentityFieldCensus = (source) => {
  const stripped = stripSwiftCommentsAndStrings(source);
  if (hasForbiddenProjectionMirrorDeclaration(stripped)) return null;
  if (coreIdentityDeclarationModifier.test(stripped)) return null;

  const declarations = [...stripped.matchAll(/\bstruct\s+CoreSessionIdentity\b/g)];
  const canonicalHeaderCount = stripped.split(canonicalCoreIdentityHeader).length - 1;
  if (declarations.length !== 1 || canonicalHeaderCount !== 1) return null;

  const tokens = swiftStructuralTokens(stripped);
  const candidates = [];
  let braceDepth = 0;
  for (let index = 0; index < tokens.length; index += 1) {
    const token = tokens[index].value;
    if (
      token === "struct" &&
      tokens[index + 1]?.value === "CoreSessionIdentity"
    ) {
      let bodyStart = index + 2;
      while (bodyStart < tokens.length && tokens[bodyStart].value !== "{") {
        if (tokens[bodyStart].value === "}") return null;
        bodyStart += 1;
      }
      candidates.push({
        declarationStart: index,
        declarationDepth: braceDepth,
        bodyStart,
      });
    }
    if (token === "{") braceDepth += 1;
    if (token === "}") {
      braceDepth -= 1;
      if (braceDepth < 0) return null;
    }
  }
  if (braceDepth !== 0 || candidates.length !== 1) return null;

  const { declarationStart, declarationDepth, bodyStart } = candidates[0];
  if (declarationDepth !== 0 || bodyStart >= tokens.length) return null;
  const header = tokens.slice(declarationStart, bodyStart).map((token) => token.value);
  if (header.join(" ") !== "struct CoreSessionIdentity : Equatable , Sendable") return null;

  let bodyEnd = bodyStart;
  let bodyDepth = 0;
  for (; bodyEnd < tokens.length; bodyEnd += 1) {
    if (tokens[bodyEnd].value === "{") bodyDepth += 1;
    if (tokens[bodyEnd].value === "}") bodyDepth -= 1;
    if (bodyDepth === 0) break;
  }
  if (bodyDepth !== 0) return null;

  const body = tokens.slice(bodyStart + 1, bodyEnd);
  const fields = [];
  let cursor = 0;
  for (const approvedField of approvedCoreIdentityFields) {
    while (body[cursor]?.value === ";") cursor += 1;
    const [name, type] = approvedField.split(":");
    if (
      body[cursor]?.value !== "let" ||
      body[cursor + 1]?.value !== name ||
      body[cursor + 2]?.value !== ":" ||
      body[cursor + 3]?.value !== type
    ) {
      return null;
    }
    fields.push(`${name}:${type}`);
    cursor += 4;
  }
  while (body[cursor]?.value === ";") cursor += 1;
  return cursor === body.length ? fields : null;
};

const coreIdentityFixture = (extraMember = "", prefix = "", suffix = "") => `
${prefix}struct CoreSessionIdentity: Equatable, Sendable {
    let userID: String
    let deviceID: String
    let homeserverURL: String
    ${extraMember}
}
${suffix}`;
for (const [fixture, expected] of [
  [
    `import Foundation
     let bait = "struct CoreSessionIdentity: Equatable, Sendable { let accessToken: String? }"
     /* extension CoreSessionIdentity { private var token: String { "bait" } } */
     ${coreIdentityFixture("// let accessToken: String?")}`,
    true,
  ],
  [coreIdentityFixture("let accessToken: String?"), false],
  [coreIdentityFixture('let refreshToken: String = "secret"'), false],
  [coreIdentityFixture("let coordinates: (String, String)"), false],
  [coreIdentityFixture("let credentials: [String: String]"), false],
  [coreIdentityFixture("let secret: Credentials.Secret"), false],
  [coreIdentityFixture("private let secret: String"), false],
  [coreIdentityFixture("var secret: String"), false],
  [coreIdentityFixture("static let secret: String"), false],
  [coreIdentityFixture('var secret: String { "secret" }'), false],
  [coreIdentityFixture("func secret() -> String { String() }"), false],
  [coreIdentityFixture("init(secret: String) {}"), false],
  [coreIdentityFixture("subscript(index: Int) -> String { String() }"), false],
  [coreIdentityFixture("typealias Secret = String"), false],
  [coreIdentityFixture("struct NestedSecret {}"), false],
  [coreIdentityFixture("", "@frozen public "), false],
  [coreIdentityFixture("", "@frozen public\n\n"), false],
  [coreIdentityFixture("", "@available(iOS 17, *)\npublic\n"), false],
  [coreIdentityFixture("", "internal\n"), false],
  [coreIdentityFixture("", "let harmless = 1; public "), false],
  [
    coreIdentityFixture(
      "",
      "",
      `extension CoreSessionIdentity { private var accessToken: String { "secret" } }`
    ),
    false,
  ],
  [
    coreIdentityFixture(
      "",
      "",
      "extension CoreSessionIdentity { struct Nested { let token: String? = nil } }"
    ),
    false,
  ],
  [
    coreIdentityFixture(
      "",
      "",
      'extension CoreSessionIdentity { var optionalToken: String? { "secret" } }'
    ),
    false,
  ],
  [
    coreIdentityFixture(
      "",
      "",
      'extension CoreSessionIdentity { static let defaultedToken: String = "secret" }'
    ),
    false,
  ],
  [coreIdentityFixture("", "", "extension\nCoreSessionIdentity {}"), false],
  [coreIdentityFixture("", "", "extension Display.CoreSessionIdentity {}"), false],
  [
    coreIdentityFixture(
      "",
      "",
      'typealias DisplayAlias = CoreSessionIdentity\nextension DisplayAlias { var accessToken: String { "secret" } }'
    ),
    false,
  ],
  [
    `let bait = "extension CoreSessionIdentity { var token: String? { \"secret\" } }"
     // extension CoreSessionIdentity { static let token = "secret" }
     // typealias DisplayAlias = CoreSessionIdentity
     ${coreIdentityFixture()}`,
    true,
  ],
]) {
  if ((coreSessionIdentityFieldCensus(fixture) !== null) !== expected) {
    throw new Error("P4-4 CoreSessionIdentity structural parser fixture failed");
  }
}

const coreIdentityFields = coreSessionIdentityFieldCensus(sessionProjectionAdapter);
if (!coreIdentityFields) {
  throw new Error(
    `P4-4 CoreSessionIdentity must be the unprefixed, module-internal closed record with only ${approvedCoreIdentityFields.join(", ")}`
  );
}
if (
  coreIdentityFields.length !== approvedCoreIdentityFields.length ||
  coreIdentityFields.some((field, index) => field !== approvedCoreIdentityFields[index])
) {
  throw new Error(
    `P4-4 CoreSessionIdentity must expose only ${approvedCoreIdentityFields.join(", ")}; found ${coreIdentityFields.join(", ")}`
  );
}

// Read every complete function header, from `func` through its body opener.
// In particular this includes a generic parameter clause before `(` and a
// trailing `where` clause, neither of which may smuggle Error across the
// display-only boundary.
const projectionMirrorFunctionHeaders = (source) => {
  const stripped = stripSwiftCommentsAndStrings(source);
  const tokens = swiftStructuralTokens(stripped);
  const headers = [];
  for (let index = 0; index < tokens.length; index += 1) {
    if (tokens[index].value !== "func" || !/^[A-Za-z_]\w*$/.test(tokens[index + 1]?.value ?? "")) continue;

    let parametersStart = index + 2;
    while (parametersStart < tokens.length && tokens[parametersStart].value !== "(") parametersStart += 1;
    if (parametersStart === tokens.length) continue;

    let parametersEnd = parametersStart;
    let parametersDepth = 0;
    for (; parametersEnd < tokens.length; parametersEnd += 1) {
      if (tokens[parametersEnd].value === "(") parametersDepth += 1;
      if (tokens[parametersEnd].value === ")") parametersDepth -= 1;
      if (parametersDepth === 0) break;
    }
    if (parametersDepth !== 0) continue;

    let bodyStart = parametersEnd + 1;
    while (bodyStart < tokens.length && tokens[bodyStart].value !== "{") {
      if (tokens[bodyStart].value === "}") break;
      bodyStart += 1;
    }
    if (bodyStart === tokens.length || tokens[bodyStart].value !== "{") continue;
    headers.push(stripped.slice(tokens[index].index, tokens[bodyStart].index));
  }
  return headers;
};
const projectionMirrorSignatureTypes = (header) => [
  ...header.replace(/^\s*func\s+\w+/, "").matchAll(/[A-Za-z_][A-Za-z0-9_]*/g),
].map((match) => match[0]);
const forbiddenMirrorTypeFragments = [
  "AuthenticatedSession",
  "Client",
  "Store",
  "Credential",
  "Token",
  "Keychain",
  "LoginRequest",
  "Password",
  "SecretVault",
  "Error",
];

// The mirror is deliberately a closed, one-file syntax surface. Typealiases
// and extensions add name-resolution or member-attachment paths that this
// lexical guard should not try to model, so the file-wide rejection above is
// intentional and includes aliases that are currently unused.
const projectionMirrorSignatureViolation = (header) => {
  if (/\b(?:re)?throws\b/.test(header)) return "throws";
  const types = projectionMirrorSignatureTypes(header);
  return types.find((type) =>
    forbiddenMirrorTypeFragments.some((fragment) => type.includes(fragment))
  );
};
const projectionMirrorFixturePasses = (source) => {
  try {
    if (hasForbiddenProjectionMirrorDeclaration(stripSwiftCommentsAndStrings(source))) {
      return false;
    }
    const headers = projectionMirrorFunctionHeaders(source);
    return headers.length === 1 && !projectionMirrorSignatureViolation(headers[0]);
  } catch {
    return false;
  }
};

const strippedProjectionAdapter = stripSwiftCommentsAndStrings(sessionProjectionAdapter);
if (hasForbiddenProjectionMirrorDeclaration(strippedProjectionAdapter)) {
  throw new Error("P4-4 projection mirror must not declare typealiases or extensions");
}
const mirrorHeaders = projectionMirrorFunctionHeaders(sessionProjectionAdapter);
const mirrorFunctionNames = mirrorHeaders
  .map((header) => header.match(/\bfunc\s+(\w+)/)?.[1])
  .filter(Boolean)
  .sort();
const approvedMirrorFunctionNames = ["closeBeforeSDKWipe", "coreSessionIdentity", "openAfterInstalledClient"];
const approvedMirrorHeaders = new Set([
  "funcopenAfterInstalledClient(userID:String,deviceID:String,homeserverURL:String,cryptoReady:Bool)async",
  "funccoreSessionIdentity()async->CoreSessionIdentity?",
  "funccloseBeforeSDKWipe()async",
]);
if (
  mirrorFunctionNames.length !== approvedMirrorFunctionNames.length ||
  mirrorFunctionNames.some((name, index) => name !== approvedMirrorFunctionNames[index])
) {
  throw new Error(
    `P4-4 projection mirror must expose only ${approvedMirrorFunctionNames.join(", ")}; found ${mirrorFunctionNames.join(", ")}`
  );
}
for (const header of mirrorHeaders) {
  const violation = projectionMirrorSignatureViolation(header);
  if (violation) {
    throw new Error(
      violation === "throws"
        ? "P4-4 projection mirror functions must not throw"
        : `P4-4 projection mirror signature/result must not use client/store/credential/Error type: ${violation}`
    );
  }
  if (!approvedMirrorHeaders.has(header.replace(/\s/g, ""))) {
    throw new Error("P4-4 projection mirror signatures must retain the exact safe string inputs and optional CoreSessionIdentity result");
  }
}
for (const [fixture, expected] of [
  ["func coreSessionIdentity() async -> CoreSessionIdentity? {", true],
  ["func poisoned(_ session: AuthenticatedSession) async -> MatrixRustSDKClientStore? {", false],
  ["func coreSessionIdentity() async throws -> CoreSessionIdentity? {", false],
  ["func poisoned<T: Error>() async -> CoreSessionIdentity? {", false],
  ["func poisoned<T>() async -> CoreSessionIdentity? where T: Swift.Error {", false],
  ["func poisoned() async -> Error? {", false],
  ["func poisoned() async -> Result<CoreSessionIdentity?, Swift.Error> {", false],
  ["func poisoned(_ failure: SessionProjectionError) async -> CoreSessionIdentity? {", false],
  [
    "typealias MirrorFailure = Swift.Error\nfunc poisoned() async -> MirrorFailure {",
    false,
  ],
  [
    "typealias ResultAlias = Result<CoreSessionIdentity?, Swift.Error>\nfunc poisoned() async -> ResultAlias {",
    false,
  ],
  [
    "typealias RootFailure = Swift.Error\ntypealias MirrorFailure = RootFailure\nfunc poisoned() async -> MirrorFailure {",
    false,
  ],
  [
    "typealias First = Second\ntypealias Second = First\nfunc coreSessionIdentity() async -> CoreSessionIdentity? {",
    false,
  ],
  [
    "typealias First = UndeclaredAlias\nfunc coreSessionIdentity() async -> CoreSessionIdentity? {",
    false,
  ],
  [
    'typealias DisplayAlias = CoreSessionIdentity\nextension DisplayAlias { var accessToken: String { "secret" } }\nfunc coreSessionIdentity() async -> CoreSessionIdentity? {',
    false,
  ],
  ["typealias Broken<T> = Swift.Error\nfunc poisoned() async -> Broken {", false],
  ["typealias MirrorFailure = Swift.\nError\nfunc poisoned() async -> MirrorFailure {", false],
  [
    '// func poisoned<T: Error>() async -> Error? {\nlet bait = "typealias MirrorFailure = Swift.Error; extension DisplayAlias {}"\nfunc coreSessionIdentity() async -> CoreSessionIdentity? {',
    true,
  ],
]) {
  if (projectionMirrorFixturePasses(fixture) !== expected) {
    throw new Error("P4-4 projection mirror signature/closed-file fixture failed");
  }
}

// GitHub Actions, not a shell-text scan of ci-build.sh, supplies the execution
// proof for this guard. Require one exact, unconditional named iOS job step so
// comments, heredocs, functions, and conditional shell snippets cannot count.
const workflowIosSteps = (source) => {
  const lines = source.split(/\r?\n/);
  const jobStart = lines.findIndex((line) => /^ {2}ios-tests:\s*$/.test(line));
  if (jobStart < 0) return [];

  let stepsStart = -1;
  for (let index = jobStart + 1; index < lines.length; index += 1) {
    if (/^ {2}[A-Za-z0-9_-]+:\s*$/.test(lines[index])) break;
    if (/^ {4}steps:\s*$/.test(lines[index])) {
      stepsStart = index;
      break;
    }
  }
  if (stepsStart < 0) return [];

  const steps = [];
  let step;
  for (let index = stepsStart + 1; index < lines.length; index += 1) {
    const line = lines[index];
    if (line.trim() && line.length - line.trimStart().length <= 4) break;
    if (/^ {6}-\s*/.test(line)) {
      step = [line];
      steps.push(step);
    } else {
      step?.push(line);
    }
  }
  return steps;
};
const meaningfulWorkflowStepLines = (step) =>
  step.map((line) => line.trim()).filter((line) => line && !line.startsWith("#"));
const isExactWorkflowCommandStep = (step, name, command) => {
  const lines = meaningfulWorkflowStepLines(step);
  return lines.length === 2 && lines[0] === `- name: ${name}` && lines[1] === `run: ${command}`;
};
const isDirectIosCiBuildStep = (step) => {
  const lines = meaningfulWorkflowStepLines(step);
  return (
    lines.includes("run: scripts/ci-build.sh") &&
    lines.includes("working-directory: synara-ios") &&
    !lines.some((line) => /^(?:if|continue-on-error):/.test(line))
  );
};
const ciWorkflowRunsScaffoldCheckBeforeIosBuild = (source) => {
  const steps = workflowIosSteps(source);
  const checkIndex = steps.findIndex((step) =>
    isExactWorkflowCommandStep(
      step,
      "Check SynaraCore Swift scaffold",
      "node scripts/check-synara-core-swift-scaffold.mjs"
    )
  );
  const buildIndex = steps.findIndex(isDirectIosCiBuildStep);
  return checkIndex >= 0 && buildIndex > checkIndex;
};
const iosWorkflowFixture = (checkStep, buildStep = `
      - name: Build and run unsigned simulator tests
        run: scripts/ci-build.sh
        working-directory: synara-ios`) => `
jobs:
  ios-tests:
    steps:
${checkStep}${buildStep}
`;
for (const [fixture, expected] of [
  [
    iosWorkflowFixture(`      - name: Check SynaraCore Swift scaffold
        run: node scripts/check-synara-core-swift-scaffold.mjs`),
    true,
  ],
  [
    iosWorkflowFixture(
      `      # - name: Check SynaraCore Swift scaffold
      #   run: node scripts/check-synara-core-swift-scaffold.mjs`
    ),
    false,
  ],
  [
    iosWorkflowFixture(`      - name: Check SynaraCore Swift scaffold
        run: |
          cat <<'EOF'
          node scripts/check-synara-core-swift-scaffold.mjs
          EOF`),
    false,
  ],
  [
    iosWorkflowFixture(`      - name: Check SynaraCore Swift scaffold
        run: |
          check_scaffold() { node scripts/check-synara-core-swift-scaffold.mjs; }`),
    false,
  ],
  [
    iosWorkflowFixture(`      - name: Check SynaraCore Swift scaffold
        if: false
        run: node scripts/check-synara-core-swift-scaffold.mjs`),
    false,
  ],
  [
    iosWorkflowFixture(`      - name: Check SynaraCore Swift scaffold
        run: "node scripts/check-synara-core-swift-scaffold.mjs"`),
    false,
  ],
  [
    `jobs:
  ios-tests:
    steps:
      - name: Build and run unsigned simulator tests
        run: scripts/ci-build.sh
        working-directory: synara-ios
      - name: Check SynaraCore Swift scaffold
        run: node scripts/check-synara-core-swift-scaffold.mjs`,
    false,
  ],
]) {
  if (ciWorkflowRunsScaffoldCheckBeforeIosBuild(fixture) !== expected) {
    throw new Error("P4-4 iOS workflow scaffold-check fixture failed");
  }
}
if (!ciWorkflowRunsScaffoldCheckBeforeIosBuild(ciWorkflow)) {
  throw new Error(
    "P4-4 CI ios-tests must directly run the named Core Swift scaffold check before synara-ios/scripts/ci-build.sh"
  );
}

// ci-build.sh retains the same command as runtime defense in depth before its
// xcodebuild calls. The exact workflow step above, rather than this text
// presence check, is what proves the guard executes in CI.
const ciBuildCheckerAssignment = 'checker="$repo_root/scripts/check-synara-core-swift-scaffold.mjs"';
const ciBuildCheckerInvocation = 'node "$checker"';
const ciBuildCheckerIndex = iosCiBuild.indexOf(ciBuildCheckerInvocation);
const ciBuildXcodebuildIndex = iosCiBuild.indexOf("\nxcodebuild");
if (
  iosCiBuild.indexOf(ciBuildCheckerAssignment) < 0 ||
  ciBuildCheckerIndex < 0 ||
  ciBuildXcodebuildIndex < 0 ||
  ciBuildCheckerIndex > ciBuildXcodebuildIndex
) {
  throw new Error("P4-4 iOS CI build must retain the Core Swift scaffold guard before xcodebuild");
}

// P4-3 is a deliberately closed session projection, not a second Matrix
// session implementation. Parse the one public record rather than scanning
// comments/tests, then fail if its field census changes or a secret field is
// ever introduced into the UniFFI surface.
const projectionRecord = udl.match(/dictionary SessionProjection \{([\s\S]*?)\};/);
if (!projectionRecord) throw new Error("missing P4-3 SessionProjection record");
const projectionFields = [...projectionRecord[1].matchAll(/^\s*(?:u64|string|boolean|SessionProjectionLifecycle)\s+(\w+);/gm)].map(
  ([, field]) => field
);
const approvedProjectionFields = [
  "generation",
  "user_id",
  "device_id",
  "homeserver_url",
  "lifecycle",
  "crypto_ready",
];
if (
  projectionFields.length !== approvedProjectionFields.length ||
  projectionFields.some((field, index) => field !== approvedProjectionFields[index])
) {
  throw new Error(
    `P4-3 SessionProjection must expose exactly ${approvedProjectionFields.join(", ")}; found ${projectionFields.join(", ")}`
  );
}
for (const forbidden of [
  "access_token",
  "refresh_token",
  "password",
  "recovery_key",
  "private_key",
  "session_key",
  "display_name",
  "avatar_url",
  "client",
  "store",
]) {
  if (projectionFields.includes(forbidden)) {
    throw new Error(`P4-3 secret/private field must not cross UniFFI: ${forbidden}`);
  }
}

const projectionObject = udl.match(/interface SessionProjectionCore \{([\s\S]*?)\};/);
if (!projectionObject) throw new Error("missing P4-3 SessionProjectionCore object");
const projectionOperations = [
  ...projectionObject[1].matchAll(/(?:constructor|void|SessionProjection\?)\s+(\w+)\s*\(/g),
].map(([, operation]) => operation);
if (projectionOperations.join(",") !== "open,session_snapshot,close") {
  throw new Error(`P4-3 facade must expose only open/session_snapshot/close; found ${projectionOperations.join(", ")}`);
}

// P4-S3d allows restore + login + attach_session_owners. Still forbid command.
const sharedCoreObject = udl.match(/interface SharedCore \{([\s\S]*?)\};/);
if (!sharedCoreObject) throw new Error("missing SharedCore object");
const sharedCoreBody = sharedCoreObject[1].replace(/\/\/.*$/gm, "");
if (!sharedCoreBody.includes("constructor();")) {
  throw new Error("SharedCore must keep the fail-closed constructor");
}
if (!sharedCoreBody.includes("constructor(IosSecretVault store)")) {
  throw new Error("P4-S3a SharedCore must accept IosSecretVault");
}
if (!sharedCoreBody.includes("restore_persisted_session")) {
  throw new Error("P4-S3b SharedCore must expose restore_persisted_session");
}
if (!sharedCoreBody.includes("login_with_password")) {
  throw new Error("P4-S3c SharedCore must expose dedicated login_with_password");
}
if (!sharedCoreBody.includes("attach_session_owners")) {
  throw new Error("P4-S3d SharedCore must expose attach_session_owners");
}
if (!sharedCoreBody.includes("room_list_snapshot")) {
  throw new Error("P4-S4 SharedCore must expose room_list_snapshot");
}
if (!sharedCoreBody.includes("invites_snapshot")) {
  throw new Error("P4-S5 SharedCore must expose invites_snapshot");
}
if (!sharedCoreBody.includes("timeline_open")) {
  throw new Error("P4-S6 SharedCore must expose timeline_open");
}
if (!sharedCoreBody.includes("timeline_close")) {
  throw new Error("P4-S6 SharedCore must expose timeline_close");
}
if (!sharedCoreBody.includes("timeline_paginate")) {
  throw new Error("P4-S6 SharedCore must expose timeline_paginate");
}
if (!sharedCoreBody.includes("typing_snapshot") || !sharedCoreBody.includes("typing_set")) {
  throw new Error("P4-S7 SharedCore must expose typing_snapshot and typing_set");
}
if (!sharedCoreBody.includes("presence_snapshot") || !sharedCoreBody.includes("presence_subscribe") || !sharedCoreBody.includes("presence_unsubscribe")) {
  throw new Error("P4-S7 SharedCore must expose presence_snapshot/subscribe/unsubscribe");
}
if (!sharedCoreBody.includes("verification_list")) {
  throw new Error("P4-S8 SharedCore must expose verification_list");
}
for (const required of ["verification_start", "verification_accept", "verification_begin_sas", "verification_confirm", "verification_mismatch", "verification_cancel", "verification_dismiss"]) {
  if (!sharedCoreBody.includes(required)) {
    throw new Error(`P4-S9 SharedCore must expose ${required}`);
  }
}
for (const required of ["device_snapshot", "device_rename", "device_delete_start", "device_delete_cancel", "backup_status", "room_key_transfer_status", "cross_signing_setup"]) {
  if (!sharedCoreBody.includes(required)) {
    throw new Error(`P4-S9-2 SharedCore must expose ${required}`);
  }
}
for (const forbidden of ["command(", "matrix_login_password", "persist_planted", "attach_typing", "invites_accept", "jump_latest", "set_read_state", "device_delete_password", "join_rule", "crypto_status", "backup_setup"]) {
  if (sharedCoreBody.includes(forbidden)) {
    throw new Error(`SharedCore must not expose ${forbidden} in P4-S9-2`);
  }
}
if (!udl.includes("callback interface IosSecretVault")) {
  throw new Error("missing IosSecretVault callback");
}
if (!udl.includes("interface IosSecretVaultError")) {
  throw new Error("missing IosSecretVaultError");
}
if (sharedCoreRestore.includes("SharedCore(store:")) {
  throw new Error("P4-S3b helper must not construct-and-drop SharedCore");
}
if (sharedCoreLogin.includes("SharedCore(store:")) {
  throw new Error("P4-S3c helper must not construct-and-drop SharedCore");
}
if (sharedCoreAttach.includes("SharedCore(store:")) {
  throw new Error("P4-S3d helper must not construct-and-drop SharedCore");
}
if (sharedCoreRoomList.includes("SharedCore(store:")) {
  throw new Error("P4-S4 helper must not construct-and-drop SharedCore");
}
if (sharedCoreInvites.includes("SharedCore(store:")) {
  throw new Error("P4-S5 helper must not construct-and-drop SharedCore");
}
if (sharedCoreTimeline.includes("SharedCore(store:")) {
  throw new Error("P4-S6 helper must not construct-and-drop SharedCore");
}
if (sharedCoreTypingPresence.includes("SharedCore(store:")) {
  throw new Error("P4-S7 helper must not construct-and-drop SharedCore");
}
if (sharedCoreVerificationList.includes("SharedCore(store:")) {
  throw new Error("P4-S8 helper must not construct-and-drop SharedCore");
}
if (sharedCoreVerificationSas.includes("SharedCore(store:")) {
  throw new Error("P4-S9 helper must not construct-and-drop SharedCore");
}
if (sharedCoreDevices.includes("SharedCore(store:")) {
  throw new Error("P4-S9-2 helper must not construct-and-drop SharedCore");
}
const roomListDto = udl.match(/dictionary RoomListSnapshotDto \{([\s\S]*?)\};/);
if (!roomListDto) throw new Error("missing RoomListSnapshotDto");
if (/\bpassword\b/.test(roomListDto[1]) || /\btoken\b/.test(roomListDto[1])) {
  throw new Error("RoomListSnapshotDto must not carry password or token fields");
}
const inviteDto = udl.match(/dictionary InviteSnapshotDto \{([\s\S]*?)\};/);
if (!inviteDto) throw new Error("missing InviteSnapshotDto");
if (/\bpassword\b/.test(inviteDto[1]) || /\btoken\b/.test(inviteDto[1])) {
  throw new Error("InviteSnapshotDto must not carry password or token fields");
}
const timelineOpenDto = udl.match(/dictionary TimelineOpenDto \{([\s\S]*?)\};/);
if (!timelineOpenDto) throw new Error("missing TimelineOpenDto");
if (/\bpassword\b/.test(timelineOpenDto[1]) || /\btoken\b/.test(timelineOpenDto[1])) {
  throw new Error("TimelineOpenDto must not carry password or token fields");
}
const timelineSnapshotDto = udl.match(/dictionary TimelineSnapshotDto \{([\s\S]*?)\};/);
if (!timelineSnapshotDto) throw new Error("missing TimelineSnapshotDto");
if (/\bpassword\b/.test(timelineSnapshotDto[1]) || /\btoken\b/.test(timelineSnapshotDto[1])) {
  throw new Error("TimelineSnapshotDto must not carry password or token fields");
}
const typingDto = udl.match(/dictionary TypingSnapshotDto \{([\s\S]*?)\};/);
if (!typingDto) throw new Error("missing TypingSnapshotDto");
if (/\bpassword\b/.test(typingDto[1]) || /\btoken\b/.test(typingDto[1])) {
  throw new Error("TypingSnapshotDto must not carry password or token fields");
}
const presenceDto = udl.match(/dictionary PresenceSnapshotDto \{([\s\S]*?)\};/);
if (!presenceDto) throw new Error("missing PresenceSnapshotDto");
if (/\bpassword\b/.test(presenceDto[1]) || /\btoken\b/.test(presenceDto[1])) {
  throw new Error("PresenceSnapshotDto must not carry password or token fields");
}
const verificationInboxDto = udl.match(/dictionary VerificationInboxDto \{([\s\S]*?)\};/);
if (!verificationInboxDto) throw new Error("missing VerificationInboxDto");
if (/\bpassword\b/.test(verificationInboxDto[1]) || /\btoken\b/.test(verificationInboxDto[1])) {
  throw new Error("VerificationInboxDto must not carry password or token fields");
}
const verificationRequestDto = udl.match(/dictionary VerificationRequestDto \{([\s\S]*?)\};/);
if (!verificationRequestDto) throw new Error("missing VerificationRequestDto");
if (/\bpassword\b/.test(verificationRequestDto[1]) || /\btoken\b/.test(verificationRequestDto[1])) {
  throw new Error("VerificationRequestDto must not carry password or token fields");
}
const verificationSasDto = udl.match(/dictionary VerificationSasDto \{([\s\S]*?)\};/);
if (!verificationSasDto) throw new Error("missing VerificationSasDto");
if (/\bpassword\b/.test(verificationSasDto[1]) || /\btoken\b/.test(verificationSasDto[1])) {
  throw new Error("VerificationSasDto must not carry password or token fields");
}
const deviceSnapshotDto = udl.match(/dictionary DeviceSnapshotDto \{([\s\S]*?)\};/);
if (!deviceSnapshotDto) throw new Error("missing DeviceSnapshotDto");
if (/\bpassword\b/.test(deviceSnapshotDto[1]) || /\btoken\b/.test(deviceSnapshotDto[1])) {
  throw new Error("DeviceSnapshotDto must not carry password or token fields");
}
const deviceDeleteDto = udl.match(/dictionary DeviceDeleteDto \{([\s\S]*?)\};/);
if (!deviceDeleteDto) throw new Error("missing DeviceDeleteDto");
if (/\bpassword\b/.test(deviceDeleteDto[1]) || /\btoken\b/.test(deviceDeleteDto[1])) {
  throw new Error("DeviceDeleteDto must not carry password or token fields");
}
const loginDto = udl.match(/dictionary SessionLoginDto \{([\s\S]*?)\};/);
if (!loginDto) throw new Error("missing SessionLoginDto");
if (/\bpassword\b/.test(loginDto[1])) {
  throw new Error("SessionLoginDto must not carry a password field");
}
const attachDto = udl.match(/dictionary SessionAttachDto \{([\s\S]*?)\};/);
if (!attachDto) throw new Error("missing SessionAttachDto");
if (/\bpassword\b/.test(attachDto[1]) || /\btoken\b/.test(attachDto[1])) {
  throw new Error("SessionAttachDto must not carry password or token fields");
}
const productionSharedCoreFfi = sharedCoreFfi.split("#[cfg(test)]")[0];
if (productionSharedCoreFfi.includes("p4-s3b-store-key")) {
  throw new Error("P4-S3b must use StoreKeyId store-key: accounts, not an invented prefix");
}

// P4-3's private Core dependency must continue satisfying every required
// Platform method without turning media configuration into a projection or a
// UniFFI surface. Check the exact inert, closed, string-free implementation.
const projectionPlatformImplStart = sessionProjectionFfi.indexOf(
  "impl Platform for ProjectionOnlyPlatform {"
);
const projectionPlatformImplEnd = sessionProjectionFfi.indexOf(
  "\n}\n\nfn inert_platform_error",
  projectionPlatformImplStart
);
const inertProjectionMediaConfig = `fn media_config(&self) -> MediaConfigFuture<'_> {
        Box::pin(async { Err(PlatformMediaConfigError::NoSession) })
    }`;
const inertProjectionMediaConfigStart = sessionProjectionFfi.indexOf(inertProjectionMediaConfig);
if (
  projectionPlatformImplStart < 0 ||
  projectionPlatformImplEnd < 0 ||
  inertProjectionMediaConfigStart < projectionPlatformImplStart ||
  inertProjectionMediaConfigStart > projectionPlatformImplEnd
) {
  throw new Error(
    "P4-3 ProjectionOnlyPlatform must implement Platform::media_config with the exact closed, string-free NoSession future"
  );
}

console.log("SynaraCore UniFFI Swift scaffold contract passed.");
