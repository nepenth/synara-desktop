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
  "synara-ios/Synara/Services/SharedCoreJoinRules.swift",
  "synara-ios/Synara/Services/SharedCoreImagePacks.swift",
  "synara-ios/Synara/Services/SharedCoreLater.swift",
  "synara-ios/Synara/Services/SharedCoreMDirect.swift",
  "synara-ios/Synara/Services/SharedCoreRoomNotes.swift",
  "synara-ios/Synara/Services/SharedCoreOwnProfile.swift",
  "synara-ios/Synara/Services/SharedCoreRoomProfile.swift",
  "synara-ios/Synara/Services/SharedCoreDirectoryVisibility.swift",
  "synara-ios/Synara/Services/SharedCoreDirectorySearch.swift",
  "synara-ios/Synara/Services/SharedCoreRoomLeaveJoin.swift",
  "synara-ios/Synara/Services/SharedCoreRoomModeration.swift",
  "synara-ios/Synara/Services/SharedCoreRoomPowerLevels.swift",
  "synara-ios/Synara/Services/SharedCoreRoomCreate.swift",
  "synara-ios/Synara/Services/SharedCoreRoomMembersSnapshots.swift",
  "synara-ios/Synara/Services/SharedCoreSpaces.swift",
  "synara-ios/Synara/Services/SharedCoreInviteActions.swift",
  "synara-ios/Synara/Services/SharedCoreTimelineReadState.swift",
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
const sharedCoreJoinRules = readFileSync(
  resolve(root, "synara-ios/Synara/Services/SharedCoreJoinRules.swift"),
  "utf8"
);
const sharedCoreImagePacks = readFileSync(
  resolve(root, "synara-ios/Synara/Services/SharedCoreImagePacks.swift"),
  "utf8"
);
const sharedCoreLater = readFileSync(
  resolve(root, "synara-ios/Synara/Services/SharedCoreLater.swift"),
  "utf8"
);
const sharedCoreMDirect = readFileSync(
  resolve(root, "synara-ios/Synara/Services/SharedCoreMDirect.swift"),
  "utf8"
);
const sharedCoreRoomNotes = readFileSync(
  resolve(root, "synara-ios/Synara/Services/SharedCoreRoomNotes.swift"),
  "utf8"
);
const sharedCoreOwnProfile = readFileSync(
  resolve(root, "synara-ios/Synara/Services/SharedCoreOwnProfile.swift"),
  "utf8"
);
const sharedCoreRoomProfile = readFileSync(
  resolve(root, "synara-ios/Synara/Services/SharedCoreRoomProfile.swift"),
  "utf8"
);
const sharedCoreDirectoryVisibility = readFileSync(
  resolve(root, "synara-ios/Synara/Services/SharedCoreDirectoryVisibility.swift"),
  "utf8"
);
const sharedCoreDirectorySearch = readFileSync(
  resolve(root, "synara-ios/Synara/Services/SharedCoreDirectorySearch.swift"),
  "utf8"
);
const sharedCoreRoomLeaveJoin = readFileSync(
  resolve(root, "synara-ios/Synara/Services/SharedCoreRoomLeaveJoin.swift"),
  "utf8"
);
const sharedCoreRoomModeration = readFileSync(
  resolve(root, "synara-ios/Synara/Services/SharedCoreRoomModeration.swift"),
  "utf8"
);
const sharedCoreRoomPowerLevels = readFileSync(
  resolve(root, "synara-ios/Synara/Services/SharedCoreRoomPowerLevels.swift"),
  "utf8"
);
const sharedCoreRoomCreate = readFileSync(
  resolve(root, "synara-ios/Synara/Services/SharedCoreRoomCreate.swift"),
  "utf8"
);
const sharedCoreRoomMembersSnapshots = readFileSync(
  resolve(root, "synara-ios/Synara/Services/SharedCoreRoomMembersSnapshots.swift"),
  "utf8"
);
const sharedCoreSpaces = readFileSync(
  resolve(root, "synara-ios/Synara/Services/SharedCoreSpaces.swift"),
  "utf8"
);
const sharedCoreInviteActions = readFileSync(
  resolve(root, "synara-ios/Synara/Services/SharedCoreInviteActions.swift"),
  "utf8"
);
const sharedCoreTimelineReadState = readFileSync(
  resolve(root, "synara-ios/Synara/Services/SharedCoreTimelineReadState.swift"),
  "utf8"
);
const sharedCoreTimelineReactions = readFileSync(
  resolve(root, "synara-ios/Synara/Services/SharedCoreTimelineReactions.swift"),
  "utf8"
);
const sharedCoreComposerReplyDraft = readFileSync(
  resolve(root, "synara-ios/Synara/Services/SharedCoreComposerReplyDraft.swift"),
  "utf8"
);
const sharedCoreSendText = readFileSync(
  resolve(root, "synara-ios/Synara/Services/SharedCoreSendText.swift"),
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
  [swiftBindingsTests, "SharedCore.newWithSecretStore(store:", "Swift P4-S3a UniFFI 0.28 named vault factory"],
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
  [udl, "DeviceSnapshotDto device_snapshot()", "P4-S9-2 SharedCore device-snapshot operation"],
  [udl, "DeviceSnapshotDto device_rename(", "P4-S9-2 SharedCore device-rename operation"],
  [udl, "DeviceDeleteDto device_delete_start(", "P4-S9-2 SharedCore delete-start operation"],
  [udl, "void device_delete_cancel(", "P4-S9-2 SharedCore delete-cancel operation"],
  [udl, "interface DeviceCommandError", "P4-S9-2 static device-family error"],
  [swiftBindingsTests, "testSharedCoreDevicesWithoutSessionFailsClosed", "Swift P4-S9-2 fail-closed device-family test"],
  [sharedCoreDevices, "deviceSnapshot", "P4-S9-2 product device-snapshot helper"],
  [sharedCoreDevices, "deviceRename", "P4-S9-2 product rename helper"],
  [sharedCoreDevices, "deviceDeleteStart", "P4-S9-2 product delete-start helper"],
  [sharedCoreDevices, "deviceDeleteCancel", "P4-S9-2 product delete-cancel helper"],
  [sharedCoreDevices, "core: SharedCore", "P4-S9-2 helper takes an already-constructed SharedCore"],
  [sharedCoreDevices, "core.deviceSnapshot", "P4-S9-2 helper reads on the caller-owned instance"],
  [sharedCoreFfi, "room_join_rule_snapshot", "P4-S9-3 typed join-rule snapshot FFI"],
  [sharedCoreFfi, "matrix_room_join_rule_snapshot", "P4-S9-3 calls the registered join-rule snapshot"],
  [udl, "RoomJoinRuleSnapshotDto room_join_rule_snapshot(", "P4-S9-3 SharedCore join-rule snapshot operation"],
  [udl, "dictionary RoomJoinRuleSnapshotDto", "P4-S9-3 privacy-safe join-rule DTO"],
  [udl, "interface JoinRuleCommandError", "P4-S9-3 static join-rule error"],
  [swiftBindingsTests, "testSharedCoreJoinRulesWithoutSessionFailsClosed", "Swift P4-S9-3 fail-closed join-rule test"],
  [sharedCoreJoinRules, "roomJoinRuleSnapshot", "P4-S9-3 product join-rule snapshot helper"],
  [sharedCoreJoinRules, "core: SharedCore", "P4-S9-3 helper takes an already-constructed SharedCore"],
  [sharedCoreJoinRules, "core.roomJoinRuleSnapshot", "P4-S9-3 helper reads on the caller-owned instance"],
  [sharedCoreFfi, "get_global_image_packs", "P4-S9-4 typed global image-pack snapshot FFI"],
  [sharedCoreFfi, "matrix_get_global_image_packs", "P4-S9-4 calls the registered global snapshot"],
  [sharedCoreFfi, "matrix_get_user_image_pack", "P4-S9-4 calls the registered user snapshot"],
  [sharedCoreFfi, "matrix_get_room_image_packs", "P4-S9-4 calls the registered room snapshot"],
  [sharedCoreFfi, "matrix_set_user_image_pack", "P4-S9-4 calls the registered user setter"],
  [sharedCoreFfi, "matrix_set_global_image_packs", "P4-S9-4 calls the registered global setter"],
  [sharedCoreFfi, "matrix_set_room_image_pack", "P4-S9-4 calls the registered room setter"],
  [udl, "GlobalImagePacksSnapshotDto get_global_image_packs()", "P4-S9-4 SharedCore global image-pack snapshot"],
  [udl, "UserImagePackSnapshotDto get_user_image_pack()", "P4-S9-4 SharedCore user image-pack snapshot"],
  [udl, "RoomImagePacksSnapshotDto get_room_image_packs(", "P4-S9-4 SharedCore room image-pack snapshot"],
  [udl, "ImagePackWriteDto set_user_image_pack(", "P4-S9-4 SharedCore user image-pack setter"],
  [udl, "ImagePackWriteDto set_global_image_packs(", "P4-S9-4 SharedCore global image-pack setter"],
  [udl, "ImagePackWriteDto set_room_image_pack(", "P4-S9-4 SharedCore room image-pack setter"],
  [udl, "dictionary ImagePackDto", "P4-S9-4 privacy-safe image-pack DTO"],
  [udl, "interface ImagePackCommandError", "P4-S9-4 static image-pack error"],
  [swiftBindingsTests, "testSharedCoreImagePacksWithoutSessionFailsClosed", "Swift P4-S9-4 fail-closed image-pack test"],
  [sharedCoreImagePacks, "getGlobalImagePacks", "P4-S9-4 product global image-pack helper"],
  [sharedCoreImagePacks, "getUserImagePack", "P4-S9-4 product user image-pack helper"],
  [sharedCoreImagePacks, "getRoomImagePacks", "P4-S9-4 product room image-pack helper"],
  [sharedCoreImagePacks, "setUserImagePack", "P4-S9-4 product user image-pack setter"],
  [sharedCoreImagePacks, "setGlobalImagePacks", "P4-S9-4 product global image-pack setter"],
  [sharedCoreImagePacks, "setRoomImagePack", "P4-S9-4 product room image-pack setter"],
  [sharedCoreImagePacks, "core: SharedCore", "P4-S9-4 helper takes an already-constructed SharedCore"],
  [sharedCoreImagePacks, "core.getGlobalImagePacks", "P4-S9-4 helper reads on the caller-owned instance"],
  [sharedCoreFfi, "later_snapshot", "P4-S9-5 typed later snapshot FFI"],
  [sharedCoreFfi, "matrix_later_snapshot", "P4-S9-5 calls the registered later snapshot"],
  [sharedCoreFfi, "matrix_later_upsert", "P4-S9-5 calls the registered later upsert"],
  [sharedCoreFfi, "matrix_later_complete", "P4-S9-5 calls the registered later complete"],
  [sharedCoreFfi, "matrix_later_snooze", "P4-S9-5 calls the registered later snooze"],
  [sharedCoreFfi, "matrix_later_clear_completed", "P4-S9-5 calls the registered later clear"],
  [sharedCoreFfi, "matrix_later_mark_reminded", "P4-S9-5 calls the registered later mark-reminded"],
  [udl, "LaterSnapshotDto later_snapshot()", "P4-S9-5 SharedCore later snapshot"],
  [udl, "LaterSnapshotDto later_upsert(", "P4-S9-5 SharedCore later upsert"],
  [udl, "LaterSnapshotDto later_complete(", "P4-S9-5 SharedCore later complete"],
  [udl, "LaterSnapshotDto later_snooze(", "P4-S9-5 SharedCore later snooze"],
  [udl, "LaterSnapshotDto later_clear_completed()", "P4-S9-5 SharedCore later clear"],
  [udl, "LaterSnapshotDto later_mark_reminded(", "P4-S9-5 SharedCore later mark-reminded"],
  [udl, "dictionary LaterItemDto", "P4-S9-5 privacy-safe later item DTO"],
  [udl, "interface LaterCommandError", "P4-S9-5 static later error"],
  [swiftBindingsTests, "testSharedCoreLaterWithoutSessionFailsClosed", "Swift P4-S9-5 fail-closed later test"],
  [sharedCoreLater, "laterSnapshot", "P4-S9-5 product later snapshot helper"],
  [sharedCoreLater, "laterUpsert", "P4-S9-5 product later upsert helper"],
  [sharedCoreLater, "laterComplete", "P4-S9-5 product later complete helper"],
  [sharedCoreLater, "laterSnooze", "P4-S9-5 product later snooze helper"],
  [sharedCoreLater, "laterClearCompleted", "P4-S9-5 product later clear helper"],
  [sharedCoreLater, "laterMarkReminded", "P4-S9-5 product later mark-reminded helper"],
  [sharedCoreLater, "core: SharedCore", "P4-S9-5 helper takes an already-constructed SharedCore"],
  [sharedCoreLater, "core.laterSnapshot", "P4-S9-5 helper reads on the caller-owned instance"],
  [sharedCoreFfi, "mdirect_snapshot", "P4-S9-6 typed m.direct snapshot FFI"],
  [sharedCoreFfi, "matrix_mdirect_snapshot", "P4-S9-6 calls the registered m.direct snapshot"],
  [sharedCoreFfi, "matrix_mdirect_add", "P4-S9-6 calls the registered m.direct add"],
  [sharedCoreFfi, "matrix_mdirect_remove", "P4-S9-6 calls the registered m.direct remove"],
  [udl, "MDirectSnapshotDto mdirect_snapshot()", "P4-S9-6 SharedCore m.direct snapshot"],
  [udl, "MDirectMutationDto mdirect_add(", "P4-S9-6 SharedCore m.direct add"],
  [udl, "MDirectMutationDto mdirect_remove(", "P4-S9-6 SharedCore m.direct remove"],
  [udl, "dictionary MDirectSnapshotDto", "P4-S9-6 privacy-safe m.direct snapshot DTO"],
  [udl, "interface MDirectCommandError", "P4-S9-6 static m.direct error"],
  [swiftBindingsTests, "testSharedCoreMDirectWithoutSessionFailsClosed", "Swift P4-S9-6 fail-closed m.direct test"],
  [sharedCoreMDirect, "mdirectSnapshot", "P4-S9-6 product m.direct snapshot helper"],
  [sharedCoreMDirect, "mdirectAdd", "P4-S9-6 product m.direct add helper"],
  [sharedCoreMDirect, "mdirectRemove", "P4-S9-6 product m.direct remove helper"],
  [sharedCoreMDirect, "core: SharedCore", "P4-S9-6 helper takes an already-constructed SharedCore"],
  [sharedCoreMDirect, "core.mdirectSnapshot", "P4-S9-6 helper reads on the caller-owned instance"],
  [sharedCoreFfi, "room_notes_snapshot", "P4-S9-7 typed room-notes snapshot FFI"],
  [sharedCoreFfi, "matrix_room_notes_snapshot", "P4-S9-7 calls the registered room-notes snapshot"],
  [sharedCoreFfi, "matrix_room_notes_upsert", "P4-S9-7 calls the registered room-notes upsert"],
  [sharedCoreFfi, "matrix_room_notes_delete", "P4-S9-7 calls the registered room-notes delete"],
  [sharedCoreFfi, "matrix_room_notes_complete_todo", "P4-S9-7 calls the registered room-notes complete"],
  [sharedCoreFfi, "matrix_room_notes_move_todo", "P4-S9-7 calls the registered room-notes move"],
  [udl, "RoomNotesSnapshotDto room_notes_snapshot()", "P4-S9-7 SharedCore room-notes snapshot"],
  [udl, "RoomNotesSnapshotDto room_notes_upsert(", "P4-S9-7 SharedCore room-notes upsert"],
  [udl, "RoomNotesSnapshotDto room_notes_delete(", "P4-S9-7 SharedCore room-notes delete"],
  [udl, "RoomNotesSnapshotDto room_notes_complete_todo(", "P4-S9-7 SharedCore room-notes complete"],
  [udl, "RoomNotesSnapshotDto room_notes_move_todo(", "P4-S9-7 SharedCore room-notes move"],
  [udl, "dictionary RoomNoteItemDto", "P4-S9-7 privacy-safe room-note item DTO"],
  [udl, "interface RoomNotesCommandError", "P4-S9-7 static room-notes error"],
  [swiftBindingsTests, "testSharedCoreRoomNotesWithoutSessionFailsClosed", "Swift P4-S9-7 fail-closed room-notes test"],
  [sharedCoreRoomNotes, "roomNotesSnapshot", "P4-S9-7 product room-notes snapshot helper"],
  [sharedCoreRoomNotes, "roomNotesUpsert", "P4-S9-7 product room-notes upsert helper"],
  [sharedCoreRoomNotes, "roomNotesDelete", "P4-S9-7 product room-notes delete helper"],
  [sharedCoreRoomNotes, "roomNotesCompleteTodo", "P4-S9-7 product room-notes complete helper"],
  [sharedCoreRoomNotes, "roomNotesMoveTodo", "P4-S9-7 product room-notes move helper"],
  [sharedCoreRoomNotes, "core: SharedCore", "P4-S9-7 helper takes an already-constructed SharedCore"],
  [sharedCoreRoomNotes, "core.roomNotesSnapshot", "P4-S9-7 helper reads on the caller-owned instance"],
  [sharedCoreFfi, "set_own_display_name", "P4-S9-8 typed own display-name FFI"],
  [sharedCoreFfi, "matrix_set_own_display_name", "P4-S9-8 calls the registered display-name command"],
  [sharedCoreFfi, "matrix_set_own_avatar", "P4-S9-8 calls the registered avatar command"],
  [udl, "OwnProfileWriteDto set_own_display_name(", "P4-S9-8 SharedCore own display-name"],
  [udl, "OwnProfileWriteDto set_own_avatar(", "P4-S9-8 SharedCore own avatar"],
  [udl, "dictionary OwnProfileWriteDto", "P4-S9-8 privacy-safe own-profile write DTO"],
  [udl, "interface OwnProfileCommandError", "P4-S9-8 static own-profile error"],
  [swiftBindingsTests, "testSharedCoreOwnProfileWithoutSessionFailsClosed", "Swift P4-S9-8 fail-closed own-profile test"],
  [sharedCoreOwnProfile, "setOwnDisplayName", "P4-S9-8 product own display-name helper"],
  [sharedCoreOwnProfile, "setOwnAvatar", "P4-S9-8 product own avatar helper"],
  [sharedCoreOwnProfile, "core: SharedCore", "P4-S9-8 helper takes an already-constructed SharedCore"],
  [sharedCoreOwnProfile, "core.setOwnDisplayName", "P4-S9-8 helper writes on the caller-owned instance"],
  [sharedCoreFfi, "set_room_name", "P4-S9-9 typed room-name FFI"],
  [sharedCoreFfi, "matrix_set_room_name", "P4-S9-9 calls the registered room-name command"],
  [sharedCoreFfi, "matrix_set_room_topic", "P4-S9-9 calls the registered room-topic command"],
  [sharedCoreFfi, "matrix_set_room_avatar", "P4-S9-9 calls the registered room-avatar command"],
  [udl, "RoomProfileWriteDto set_room_name(", "P4-S9-9 SharedCore room name"],
  [udl, "RoomProfileWriteDto set_room_topic(", "P4-S9-9 SharedCore room topic"],
  [udl, "RoomProfileWriteDto set_room_avatar(", "P4-S9-9 SharedCore room avatar"],
  [udl, "dictionary RoomProfileWriteDto", "P4-S9-9 privacy-safe room-profile write DTO"],
  [udl, "interface RoomProfileCommandError", "P4-S9-9 static room-profile error"],
  [swiftBindingsTests, "testSharedCoreRoomProfileWithoutSessionFailsClosed", "Swift P4-S9-9 fail-closed room-profile test"],
  [sharedCoreRoomProfile, "setRoomName", "P4-S9-9 product room-name helper"],
  [sharedCoreRoomProfile, "setRoomTopic", "P4-S9-9 product room-topic helper"],
  [sharedCoreRoomProfile, "setRoomAvatar", "P4-S9-9 product room-avatar helper"],
  [sharedCoreRoomProfile, "core: SharedCore", "P4-S9-9 helper takes an already-constructed SharedCore"],
  [sharedCoreRoomProfile, "core.setRoomName", "P4-S9-9 helper writes on the caller-owned instance"],
  [sharedCoreFfi, "get_room_directory_visibility", "P4-S9-10 typed directory-visibility get FFI"],
  [sharedCoreFfi, "matrix_get_room_directory_visibility", "P4-S9-10 calls the registered directory-visibility get command"],
  [sharedCoreFfi, "matrix_set_room_directory_visibility", "P4-S9-10 calls the registered directory-visibility set command"],
  [udl, "RoomDirectoryVisibilityDto get_room_directory_visibility(", "P4-S9-10 SharedCore directory-visibility get"],
  [udl, "RoomDirectoryVisibilityWriteDto set_room_directory_visibility(", "P4-S9-10 SharedCore directory-visibility set"],
  [udl, "dictionary RoomDirectoryVisibilityDto", "P4-S9-10 privacy-safe directory-visibility read DTO"],
  [udl, "interface DirectoryVisibilityCommandError", "P4-S9-10 static directory-visibility error"],
  [swiftBindingsTests, "testSharedCoreDirectoryVisibilityWithoutSessionFailsClosed", "Swift P4-S9-10 fail-closed directory-visibility test"],
  [sharedCoreDirectoryVisibility, "getRoomDirectoryVisibility", "P4-S9-10 product directory-visibility get helper"],
  [sharedCoreDirectoryVisibility, "setRoomDirectoryVisibility", "P4-S9-10 product directory-visibility set helper"],
  [sharedCoreDirectoryVisibility, "core: SharedCore", "P4-S9-10 helper takes an already-constructed SharedCore"],
  [sharedCoreDirectoryVisibility, "core.getRoomDirectoryVisibility", "P4-S9-10 helper reads on the caller-owned instance"],
  [sharedCoreFfi, "room_directory_protocols", "P4-S9-11 typed directory-protocols FFI"],
  [sharedCoreFfi, "matrix_room_directory_protocols", "P4-S9-11 calls the registered directory-protocols command"],
  [sharedCoreFfi, "matrix_room_directory_search", "P4-S9-11 calls the registered directory-search command"],
  [sharedCoreFfi, "matrix_room_directory_cancel", "P4-S9-11 calls the registered directory-cancel command"],
  [udl, "RoomDirectoryProtocolsDto room_directory_protocols(", "P4-S9-11 SharedCore directory protocols"],
  [udl, "RoomDirectorySearchDto room_directory_search(", "P4-S9-11 SharedCore directory search"],
  [udl, "RoomDirectorySearchDto room_directory_cancel(", "P4-S9-11 SharedCore directory cancel"],
  [udl, "dictionary RoomDirectorySearchDto", "P4-S9-11 privacy-safe directory-search DTO"],
  [udl, "interface DirectorySearchCommandError", "P4-S9-11 static directory-search error"],
  [swiftBindingsTests, "testSharedCoreDirectorySearchWithoutSessionFailsClosed", "Swift P4-S9-11 fail-closed directory-search test"],
  [sharedCoreDirectorySearch, "roomDirectoryProtocols", "P4-S9-11 product directory-protocols helper"],
  [sharedCoreDirectorySearch, "roomDirectorySearch", "P4-S9-11 product directory-search helper"],
  [sharedCoreDirectorySearch, "roomDirectoryCancel", "P4-S9-11 product directory-cancel helper"],
  [sharedCoreDirectorySearch, "core: SharedCore", "P4-S9-11 helper takes an already-constructed SharedCore"],
  [sharedCoreDirectorySearch, "core.roomDirectorySearch", "P4-S9-11 helper searches on the caller-owned instance"],
  [sharedCoreFfi, "room_leave", "P4-S9-12 typed room-leave FFI"],
  [sharedCoreFfi, "matrix_room_leave", "P4-S9-12 calls the registered room-leave command"],
  [sharedCoreFfi, "matrix_room_join", "P4-S9-12 calls the registered room-join command"],
  [udl, "RoomMembershipWriteDto room_leave(", "P4-S9-12 SharedCore room leave"],
  [udl, "RoomMembershipWriteDto room_join(", "P4-S9-12 SharedCore room join"],
  [udl, "dictionary RoomMembershipWriteDto", "P4-S9-12 privacy-safe room-membership write DTO"],
  [udl, "interface RoomMembershipCommandError", "P4-S9-12 static room-membership error"],
  [swiftBindingsTests, "testSharedCoreRoomLeaveJoinWithoutSessionFailsClosed", "Swift P4-S9-12 fail-closed room leave/join test"],
  [sharedCoreRoomLeaveJoin, "roomLeave", "P4-S9-12 product room-leave helper"],
  [sharedCoreRoomLeaveJoin, "roomJoin", "P4-S9-12 product room-join helper"],
  [sharedCoreRoomLeaveJoin, "core: SharedCore", "P4-S9-12 helper takes an already-constructed SharedCore"],
  [sharedCoreRoomLeaveJoin, "core.roomLeave", "P4-S9-12 helper leaves on the caller-owned instance"],
  [sharedCoreFfi, "room_invite", "P4-S9-13 typed room-invite FFI"],
  [sharedCoreFfi, "matrix_room_invite", "P4-S9-13 calls the registered room-invite command"],
  [sharedCoreFfi, "matrix_room_kick", "P4-S9-13 calls the registered room-kick command"],
  [sharedCoreFfi, "matrix_room_ban", "P4-S9-13 calls the registered room-ban command"],
  [sharedCoreFfi, "matrix_room_unban", "P4-S9-13 calls the registered room-unban command"],
  [udl, "RoomModerationWriteDto room_invite(", "P4-S9-13 SharedCore room invite"],
  [udl, "RoomModerationWriteDto room_kick(", "P4-S9-13 SharedCore room kick"],
  [udl, "RoomModerationWriteDto room_ban(", "P4-S9-13 SharedCore room ban"],
  [udl, "RoomModerationWriteDto room_unban(", "P4-S9-13 SharedCore room unban"],
  [udl, "dictionary RoomModerationWriteDto", "P4-S9-13 privacy-safe room-moderation write DTO"],
  [udl, "interface RoomModerationCommandError", "P4-S9-13 static room-moderation error"],
  [swiftBindingsTests, "testSharedCoreRoomModerationWithoutSessionFailsClosed", "Swift P4-S9-13 fail-closed room moderation test"],
  [sharedCoreRoomModeration, "roomInvite", "P4-S9-13 product room-invite helper"],
  [sharedCoreRoomModeration, "roomKick", "P4-S9-13 product room-kick helper"],
  [sharedCoreRoomModeration, "roomBan", "P4-S9-13 product room-ban helper"],
  [sharedCoreRoomModeration, "roomUnban", "P4-S9-13 product room-unban helper"],
  [sharedCoreRoomModeration, "core: SharedCore", "P4-S9-13 helper takes an already-constructed SharedCore"],
  [sharedCoreRoomModeration, "core.roomInvite", "P4-S9-13 helper invites on the caller-owned instance"],
  [sharedCoreFfi, "room_set_power_level", "P4-S9-14 typed room-set-power-level FFI"],
  [sharedCoreFfi, "matrix_room_set_power_level", "P4-S9-14 calls the registered set-power-level command"],
  [sharedCoreFfi, "matrix_room_set_power_levels", "P4-S9-14 calls the registered set-power-levels command"],
  [sharedCoreFfi, "matrix_room_set_power_level_tags", "P4-S9-14 calls the registered set-power-level-tags command"],
  [udl, "RoomPowerLevelWriteDto room_set_power_level(", "P4-S9-14 SharedCore set power level"],
  [udl, "RoomPowerLevelWriteDto room_set_power_levels(", "P4-S9-14 SharedCore set power levels"],
  [udl, "RoomPowerLevelWriteDto room_set_power_level_tags(", "P4-S9-14 SharedCore set power-level tags"],
  [udl, "dictionary RoomPowerLevelWriteDto", "P4-S9-14 privacy-safe room-power-level write DTO"],
  [udl, "interface RoomPowerLevelCommandError", "P4-S9-14 static room-power-level error"],
  [swiftBindingsTests, "testSharedCoreRoomPowerLevelsWithoutSessionFailsClosed", "Swift P4-S9-14 fail-closed room power-level test"],
  [sharedCoreRoomPowerLevels, "roomSetPowerLevel", "P4-S9-14 product set-power-level helper"],
  [sharedCoreRoomPowerLevels, "roomSetPowerLevels", "P4-S9-14 product set-power-levels helper"],
  [sharedCoreRoomPowerLevels, "roomSetPowerLevelTags", "P4-S9-14 product set-power-level-tags helper"],
  [sharedCoreRoomPowerLevels, "core: SharedCore", "P4-S9-14 helper takes an already-constructed SharedCore"],
  [sharedCoreRoomPowerLevels, "core.roomSetPowerLevel", "P4-S9-14 helper writes on the caller-owned instance"],
  [readFileSync(resolve(root, "synara-ios/Synara.xcodeproj/project.pbxproj"), "utf8"), "SharedCoreRoomPowerLevels.swift in Sources", "P4-S9-14 helper in Xcode target"],
  [sharedCoreFfi, "room_create", "P4-S9-15 typed room-create FFI"],
  [sharedCoreFfi, "matrix_room_create", "P4-S9-15 calls the registered room-create command"],
  [udl, "RoomCreateDto room_create(", "P4-S9-15 SharedCore room create"],
  [udl, "dictionary RoomCreateRequestDto", "P4-S9-15 typed room-create request DTO"],
  [udl, "dictionary RoomCreateDto", "P4-S9-15 privacy-safe room-create result DTO"],
  [udl, "interface RoomCreateCommandError", "P4-S9-15 static room-create error"],
  [swiftBindingsTests, "testSharedCoreRoomCreateWithoutSessionFailsClosed", "Swift P4-S9-15 fail-closed room-create test"],
  [sharedCoreRoomCreate, "roomCreate", "P4-S9-15 product room-create helper"],
  [sharedCoreRoomCreate, "core: SharedCore", "P4-S9-15 helper takes an already-constructed SharedCore"],
  [sharedCoreRoomCreate, "core.roomCreate", "P4-S9-15 helper creates on the caller-owned instance"],
  [readFileSync(resolve(root, "synara-ios/Synara.xcodeproj/project.pbxproj"), "utf8"), "SharedCoreRoomCreate.swift in Sources", "P4-S9-15 helper in Xcode target"],
  [sharedCoreFfi, "room_members_snapshot", "P4-S9-16 typed members-snapshot FFI"],
  [sharedCoreFfi, "matrix_room_members_snapshot", "P4-S9-16 calls the registered members-snapshot command"],
  [sharedCoreFfi, "matrix_room_power_levels_snapshot", "P4-S9-16 calls the registered power-levels-snapshot command"],
  [sharedCoreFfi, "matrix_room_creators_snapshot", "P4-S9-16 calls the registered creators-snapshot command"],
  [sharedCoreFfi, "matrix_room_power_level_tags_snapshot", "P4-S9-16 calls the registered power-level-tags-snapshot command"],
  [udl, "RoomMembersSnapshotDto room_members_snapshot(", "P4-S9-16 SharedCore members snapshot"],
  [udl, "RoomPowerLevelsSnapshotDto room_power_levels_snapshot(", "P4-S9-16 SharedCore power-levels snapshot"],
  [udl, "RoomCreatorsSnapshotDto room_creators_snapshot(", "P4-S9-16 SharedCore creators snapshot"],
  [udl, "RoomPowerLevelTagsSnapshotDto room_power_level_tags_snapshot(", "P4-S9-16 SharedCore power-level-tags snapshot"],
  [udl, "dictionary RoomMembersSnapshotDto", "P4-S9-16 privacy-safe members snapshot DTO"],
  [udl, "dictionary RoomPowerLevelsSnapshotDto", "P4-S9-16 privacy-safe power-levels snapshot DTO"],
  [udl, "dictionary RoomCreatorsSnapshotDto", "P4-S9-16 privacy-safe creators snapshot DTO"],
  [udl, "dictionary RoomPowerLevelTagsSnapshotDto", "P4-S9-16 privacy-safe power-level-tags snapshot DTO"],
  [udl, "interface RoomMembersSnapshotError", "P4-S9-16 static members-snapshot error"],
  [swiftBindingsTests, "testSharedCoreRoomMembersSnapshotsWithoutSessionFailsClosed", "Swift P4-S9-16 fail-closed members-snapshot test"],
  [sharedCoreRoomMembersSnapshots, "roomMembersSnapshot", "P4-S9-16 product members-snapshot helper"],
  [sharedCoreRoomMembersSnapshots, "roomPowerLevelsSnapshot", "P4-S9-16 product power-levels-snapshot helper"],
  [sharedCoreRoomMembersSnapshots, "roomCreatorsSnapshot", "P4-S9-16 product creators-snapshot helper"],
  [sharedCoreRoomMembersSnapshots, "roomPowerLevelTagsSnapshot", "P4-S9-16 product power-level-tags-snapshot helper"],
  [sharedCoreRoomMembersSnapshots, "core: SharedCore", "P4-S9-16 helper takes an already-constructed SharedCore"],
  [sharedCoreRoomMembersSnapshots, "core.roomMembersSnapshot", "P4-S9-16 helper reads on the caller-owned instance"],
  [readFileSync(resolve(root, "synara-ios/Synara.xcodeproj/project.pbxproj"), "utf8"), "SharedCoreRoomMembersSnapshots.swift in Sources", "P4-S9-16 helper in Xcode target"],
  [sharedCoreFfi, "space_parents_snapshot", "P4-S9-17 typed space-parents-snapshot FFI"],
  [sharedCoreFfi, "matrix_space_parents_snapshot", "P4-S9-17 calls the registered space-parents-snapshot command"],
  [sharedCoreFfi, "matrix_space_hierarchy_snapshot", "P4-S9-17 calls the registered space-hierarchy-snapshot command"],
  [sharedCoreFfi, "matrix_space_children_snapshot", "P4-S9-17 calls the registered space-children-snapshot command"],
  [sharedCoreFfi, "matrix_space_child_set", "P4-S9-17 calls the registered space-child-set command"],
  [sharedCoreFfi, "matrix_space_child_remove", "P4-S9-17 calls the registered space-child-remove command"],
  [sharedCoreFfi, "matrix_restricted_join_reparent", "P4-S9-17 calls the registered restricted-join-reparent command"],
  [udl, "SpaceParentsSnapshotDto space_parents_snapshot(", "P4-S9-17 SharedCore space parents snapshot"],
  [udl, "SpaceHierarchySnapshotDto space_hierarchy_snapshot(", "P4-S9-17 SharedCore space hierarchy snapshot"],
  [udl, "SpaceChildrenSnapshotDto space_children_snapshot(", "P4-S9-17 SharedCore space children snapshot"],
  [udl, "SpaceChildMutationDto space_child_set(", "P4-S9-17 SharedCore space child set"],
  [udl, "SpaceChildMutationDto space_child_remove(", "P4-S9-17 SharedCore space child remove"],
  [udl, "RestrictedJoinReparentDto restricted_join_reparent(", "P4-S9-17 SharedCore restricted-join reparent"],
  [udl, "dictionary SpaceParentsSnapshotDto", "P4-S9-17 privacy-safe space parents snapshot DTO"],
  [udl, "dictionary SpaceHierarchySnapshotDto", "P4-S9-17 privacy-safe space hierarchy snapshot DTO"],
  [udl, "dictionary SpaceChildrenSnapshotDto", "P4-S9-17 privacy-safe space children snapshot DTO"],
  [udl, "dictionary SpaceChildMutationDto", "P4-S9-17 privacy-safe space-child mutation DTO"],
  [udl, "dictionary RestrictedJoinReparentDto", "P4-S9-17 privacy-safe restricted-join reparent DTO"],
  [udl, "interface SpaceCommandError", "P4-S9-17 static space error"],
  [swiftBindingsTests, "testSharedCoreSpacesWithoutSessionFailsClosed", "Swift P4-S9-17 fail-closed spaces test"],
  [sharedCoreSpaces, "spaceParentsSnapshot", "P4-S9-17 product space-parents-snapshot helper"],
  [sharedCoreSpaces, "spaceHierarchySnapshot", "P4-S9-17 product space-hierarchy-snapshot helper"],
  [sharedCoreSpaces, "spaceChildrenSnapshot", "P4-S9-17 product space-children-snapshot helper"],
  [sharedCoreSpaces, "spaceChildSet", "P4-S9-17 product space-child-set helper"],
  [sharedCoreSpaces, "spaceChildRemove", "P4-S9-17 product space-child-remove helper"],
  [sharedCoreSpaces, "restrictedJoinReparent", "P4-S9-17 product restricted-join-reparent helper"],
  [sharedCoreSpaces, "core: SharedCore", "P4-S9-17 helper takes an already-constructed SharedCore"],
  [sharedCoreSpaces, "core.spaceParentsSnapshot", "P4-S9-17 helper reads on the caller-owned instance"],
  [readFileSync(resolve(root, "synara-ios/Synara.xcodeproj/project.pbxproj"), "utf8"), "SharedCoreSpaces.swift in Sources", "P4-S9-17 helper in Xcode target"],
  [sharedCoreFfi, "invites_accept", "P4-S9-18 typed invites-accept FFI"],
  [sharedCoreFfi, "matrix_invites_accept", "P4-S9-18 calls the registered invites-accept command"],
  [sharedCoreFfi, "matrix_invites_decline", "P4-S9-18 calls the registered invites-decline command"],
  [sharedCoreFfi, "matrix_invites_report_spam", "P4-S9-18 calls the registered invites-report-spam command"],
  [sharedCoreFfi, "matrix_invites_block_sender", "P4-S9-18 calls the registered invites-block-sender command"],
  [udl, "InviteSnapshotDto invites_accept(", "P4-S9-18 SharedCore invite accept"],
  [udl, "InviteSnapshotDto invites_decline(", "P4-S9-18 SharedCore invite decline"],
  [udl, "InviteSnapshotDto invites_report_spam(", "P4-S9-18 SharedCore invite report-spam"],
  [udl, "InviteSnapshotDto invites_block_sender(", "P4-S9-18 SharedCore invite block-sender"],
  [udl, "interface InviteActionError", "P4-S9-18 static invite-action error"],
  [swiftBindingsTests, "testSharedCoreInviteActionsWithoutSessionFailsClosed", "Swift P4-S9-18 fail-closed invite-action test"],
  [sharedCoreInviteActions, "invitesAccept", "P4-S9-18 product invite-accept helper"],
  [sharedCoreInviteActions, "invitesDecline", "P4-S9-18 product invite-decline helper"],
  [sharedCoreInviteActions, "invitesReportSpam", "P4-S9-18 product invite-report-spam helper"],
  [sharedCoreInviteActions, "invitesBlockSender", "P4-S9-18 product invite-block-sender helper"],
  [sharedCoreInviteActions, "core: SharedCore", "P4-S9-18 helper takes an already-constructed SharedCore"],
  [sharedCoreInviteActions, "core.invitesAccept", "P4-S9-18 helper writes on the caller-owned instance"],
  [readFileSync(resolve(root, "synara-ios/Synara.xcodeproj/project.pbxproj"), "utf8"), "SharedCoreInviteActions.swift in Sources", "P4-S9-18 helper in Xcode target"],
  [sharedCoreFfi, "timeline_event_readback", "P4-S9-19 typed event-readback FFI"],
  [sharedCoreFfi, "matrix_timeline_event_readback", "P4-S9-19 calls the registered event-readback command"],
  [sharedCoreFfi, "matrix_timeline_set_read_state", "P4-S9-19 calls the registered set-read-state command"],
  [sharedCoreFfi, "matrix_timeline_jump_latest", "P4-S9-19 calls the registered jump-latest command"],
  [udl, "TimelineEventReadbackDto timeline_event_readback(", "P4-S9-19 SharedCore event readback"],
  [udl, "TimelineReadStateDto timeline_set_read_state(", "P4-S9-19 SharedCore set-read-state"],
  [udl, "TimelineOpenDto timeline_jump_latest(", "P4-S9-19 SharedCore jump-latest"],
  [udl, "interface TimelineReadStateError", "P4-S9-19 static timeline read-state error"],
  [swiftBindingsTests, "testSharedCoreTimelineReadStateWithoutSessionFailsClosed", "Swift P4-S9-19 fail-closed timeline read-state test"],
  [sharedCoreTimelineReadState, "timelineEventReadback", "P4-S9-19 product event-readback helper"],
  [sharedCoreTimelineReadState, "timelineSetReadState", "P4-S9-19 product set-read-state helper"],
  [sharedCoreTimelineReadState, "timelineJumpLatest", "P4-S9-19 product jump-latest helper"],
  [sharedCoreTimelineReadState, "core: SharedCore", "P4-S9-19 helper takes an already-constructed SharedCore"],
  [sharedCoreTimelineReadState, "core.timelineEventReadback", "P4-S9-19 helper reads on the caller-owned instance"],
  [readFileSync(resolve(root, "synara-ios/Synara.xcodeproj/project.pbxproj"), "utf8"), "SharedCoreTimelineReadState.swift in Sources", "P4-S9-19 helper in Xcode target"],
  [sharedCoreFfi, "reaction_ensure", "P4-S9-20 typed reaction-ensure FFI"],
  [sharedCoreFfi, "matrix_reaction_ensure", "P4-S9-20 calls the registered reaction-ensure command"],
  [sharedCoreFfi, "matrix_reaction_redact", "P4-S9-20 calls the registered reaction-redact command"],
  [sharedCoreFfi, "matrix_timeline_reaction_toggle", "P4-S9-20 calls the registered reaction-toggle command"],
  [udl, "TimelineReactionMutationDto reaction_ensure(", "P4-S9-20 SharedCore reaction ensure"],
  [udl, "TimelineReactionMutationDto reaction_redact(", "P4-S9-20 SharedCore reaction redact"],
  [udl, "TimelineReactionMutationDto timeline_reaction_toggle(", "P4-S9-20 SharedCore reaction toggle"],
  [udl, "interface TimelineReactionError", "P4-S9-20 static timeline reaction error"],
  [swiftBindingsTests, "testSharedCoreTimelineReactionsWithoutSessionFailsClosed", "Swift P4-S9-20 fail-closed timeline reaction test"],
  [sharedCoreTimelineReactions, "reactionEnsure", "P4-S9-20 product reaction-ensure helper"],
  [sharedCoreTimelineReactions, "reactionRedact", "P4-S9-20 product reaction-redact helper"],
  [sharedCoreTimelineReactions, "timelineReactionToggle", "P4-S9-20 product reaction-toggle helper"],
  [sharedCoreTimelineReactions, "core: SharedCore", "P4-S9-20 helper takes an already-constructed SharedCore"],
  [sharedCoreTimelineReactions, "core.reactionEnsure", "P4-S9-20 helper writes on the caller-owned instance"],
  [readFileSync(resolve(root, "synara-ios/Synara.xcodeproj/project.pbxproj"), "utf8"), "SharedCoreTimelineReactions.swift in Sources", "P4-S9-20 helper in Xcode target"],
  [sharedCoreFfi, "composer_set_reply_draft", "P4-S9-21 typed composer-set-reply-draft FFI"],
  [sharedCoreFfi, "matrix_composer_set_reply_draft", "P4-S9-21 calls the registered composer-set command"],
  [sharedCoreFfi, "matrix_composer_get_reply_draft", "P4-S9-21 calls the registered composer-get command"],
  [sharedCoreFfi, "matrix_composer_clear_reply_draft", "P4-S9-21 calls the registered composer-clear command"],
  [udl, "ComposerReplyDraftDto composer_set_reply_draft(", "P4-S9-21 SharedCore composer set reply draft"],
  [udl, "ComposerReplyDraftDto composer_get_reply_draft(", "P4-S9-21 SharedCore composer get reply draft"],
  [udl, "ComposerReplyDraftDto composer_clear_reply_draft(", "P4-S9-21 SharedCore composer clear reply draft"],
  [udl, "interface ComposerReplyDraftError", "P4-S9-21 static composer reply-draft error"],
  [swiftBindingsTests, "testSharedCoreComposerReplyDraftWithoutSessionFailsClosed", "Swift P4-S9-21 fail-closed composer reply-draft test"],
  [sharedCoreComposerReplyDraft, "composerSetReplyDraft", "P4-S9-21 product composer-set helper"],
  [sharedCoreComposerReplyDraft, "composerGetReplyDraft", "P4-S9-21 product composer-get helper"],
  [sharedCoreComposerReplyDraft, "composerClearReplyDraft", "P4-S9-21 product composer-clear helper"],
  [sharedCoreComposerReplyDraft, "core: SharedCore", "P4-S9-21 helper takes an already-constructed SharedCore"],
  [sharedCoreComposerReplyDraft, "core.composerSetReplyDraft", "P4-S9-21 helper writes on the caller-owned instance"],
  [readFileSync(resolve(root, "synara-ios/Synara.xcodeproj/project.pbxproj"), "utf8"), "SharedCoreComposerReplyDraft.swift in Sources", "P4-S9-21 helper in Xcode target"],
  [sharedCoreFfi, "send_text", "P4-S9-22 typed send-text FFI"],
  [sharedCoreFfi, "matrix_send_text", "P4-S9-22 calls the registered send-text command"],
  [udl, "SendTextDto send_text(", "P4-S9-22 SharedCore send text"],
  [udl, "interface SendTextError", "P4-S9-22 static send-text error"],
  [swiftBindingsTests, "testSharedCoreSendTextWithoutSessionFailsClosed", "Swift P4-S9-22 fail-closed send-text test"],
  [sharedCoreSendText, "sendText", "P4-S9-22 product send-text helper"],
  [sharedCoreSendText, "core: SharedCore", "P4-S9-22 helper takes an already-constructed SharedCore"],
  [sharedCoreSendText, "core.sendText", "P4-S9-22 helper writes on the caller-owned instance"],
  [readFileSync(resolve(root, "synara-ios/Synara.xcodeproj/project.pbxproj"), "utf8"), "SharedCoreSendText.swift in Sources", "P4-S9-22 helper in Xcode target"],
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

// P4-S9-6 allows restore + login + attach + consume wrappers through m.direct. Still forbid generic command.
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
if (!sharedCoreBody.includes('[Name="new_with_secret_store"]')) {
  throw new Error("P4-S3a vault constructor must stay a named UniFFI factory");
}
if (swiftBindingsTests.includes("SharedCore(store:")) {
  throw new Error(
    "UniFFI 0.28 Swift has no SharedCore(store:) init; use SharedCore.newWithSecretStore(store:)"
  );
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
for (const required of ["device_snapshot", "device_rename", "device_delete_start", "device_delete_cancel"]) {
  if (!sharedCoreBody.includes(required)) {
    throw new Error(`P4-S9-2 SharedCore must expose ${required}`);
  }
}
if (!sharedCoreBody.includes("room_join_rule_snapshot")) {
  throw new Error("P4-S9-3 SharedCore must expose room_join_rule_snapshot");
}
for (const required of ["get_global_image_packs", "get_user_image_pack", "get_room_image_packs", "set_user_image_pack", "set_global_image_packs", "set_room_image_pack"]) {
  if (!sharedCoreBody.includes(required)) {
    throw new Error(`P4-S9-4 SharedCore must expose ${required}`);
  }
}
for (const required of ["later_snapshot", "later_upsert", "later_complete", "later_snooze", "later_clear_completed", "later_mark_reminded"]) {
  if (!sharedCoreBody.includes(required)) {
    throw new Error(`P4-S9-5 SharedCore must expose ${required}`);
  }
}
for (const required of ["mdirect_snapshot", "mdirect_add", "mdirect_remove"]) {
  if (!sharedCoreBody.includes(required)) {
    throw new Error(`P4-S9-6 SharedCore must expose ${required}`);
  }
}
for (const required of ["room_notes_snapshot", "room_notes_upsert", "room_notes_delete", "room_notes_complete_todo", "room_notes_move_todo"]) {
  if (!sharedCoreBody.includes(required)) {
    throw new Error(`P4-S9-7 SharedCore must expose ${required}`);
  }
}
for (const required of ["set_own_display_name", "set_own_avatar"]) {
  if (!sharedCoreBody.includes(required)) {
    throw new Error(`P4-S9-8 SharedCore must expose ${required}`);
  }
}
for (const required of ["set_room_name", "set_room_topic", "set_room_avatar"]) {
  if (!sharedCoreBody.includes(required)) {
    throw new Error(`P4-S9-9 SharedCore must expose ${required}`);
  }
}
for (const required of ["get_room_directory_visibility", "set_room_directory_visibility"]) {
  if (!sharedCoreBody.includes(required)) {
    throw new Error(`P4-S9-10 SharedCore must expose ${required}`);
  }
}
for (const required of ["room_directory_protocols", "room_directory_search", "room_directory_cancel"]) {
  if (!sharedCoreBody.includes(required)) {
    throw new Error(`P4-S9-11 SharedCore must expose ${required}`);
  }
}
for (const required of ["room_leave", "room_join("]) {
  if (!sharedCoreBody.includes(required)) {
    throw new Error(`P4-S9-12 SharedCore must expose ${required}`);
  }
}
for (const required of ["room_invite", "room_kick", "room_ban", "room_unban"]) {
  if (!sharedCoreBody.includes(required)) {
    throw new Error(`P4-S9-13 SharedCore must expose ${required}`);
  }
}
for (const required of ["room_set_power_level(", "room_set_power_levels(", "room_set_power_level_tags("]) {
  if (!sharedCoreBody.includes(required)) {
    throw new Error(`P4-S9-14 SharedCore must expose ${required}`);
  }
}
for (const required of ["room_create("]) {
  if (!sharedCoreBody.includes(required)) {
    throw new Error(`P4-S9-15 SharedCore must expose ${required}`);
  }
}
for (const required of ["room_members_snapshot(", "room_power_levels_snapshot(", "room_creators_snapshot(", "room_power_level_tags_snapshot("]) {
  if (!sharedCoreBody.includes(required)) {
    throw new Error(`P4-S9-16 SharedCore must expose ${required}`);
  }
}
for (const required of ["space_parents_snapshot(", "space_hierarchy_snapshot(", "space_children_snapshot(", "space_child_set(", "space_child_remove(", "restricted_join_reparent("]) {
  if (!sharedCoreBody.includes(required)) {
    throw new Error(`P4-S9-17 SharedCore must expose ${required}`);
  }
}
for (const required of ["invites_accept(", "invites_decline(", "invites_report_spam(", "invites_block_sender("]) {
  if (!sharedCoreBody.includes(required)) {
    throw new Error(`P4-S9-18 SharedCore must expose ${required}`);
  }
}
for (const required of ["timeline_event_readback(", "timeline_set_read_state(", "timeline_jump_latest("]) {
  if (!sharedCoreBody.includes(required)) {
    throw new Error(`P4-S9-19 SharedCore must expose ${required}`);
  }
}
for (const required of ["reaction_ensure(", "reaction_redact(", "timeline_reaction_toggle("]) {
  if (!sharedCoreBody.includes(required)) {
    throw new Error(`P4-S9-20 SharedCore must expose ${required}`);
  }
}
for (const required of ["composer_set_reply_draft(", "composer_get_reply_draft(", "composer_clear_reply_draft("]) {
  if (!sharedCoreBody.includes(required)) {
    throw new Error(`P4-S9-21 SharedCore must expose ${required}`);
  }
}
for (const required of ["send_text("]) {
  if (!sharedCoreBody.includes(required)) {
    throw new Error(`P4-S9-22 SharedCore must expose ${required}`);
  }
}
for (const forbidden of ["command(", "matrix_login_password", "persist_planted", "attach_typing", "send_sticker", "send_poll", "edit_message", "poll_respond", "matrix_send_sticker", "matrix_send_poll", "matrix_edit_message", "matrix_poll_respond", "device_delete_password", "backup_status", "room_key_transfer_status", "cross_signing_setup", "set_room_join_rule", "crypto_status", "backup_setup"]) {
  if (sharedCoreBody.includes(forbidden)) {
    throw new Error(`SharedCore must not expose ${forbidden} in P4-S9-22`);
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
for (const forbidden of ["backupStatus", "roomKeyTransferStatus", "crossSigningSetup"]) {
  if (sharedCoreDevices.includes(forbidden)) {
    throw new Error(`P4-S9-2 helper must not wrap leftover-adjacent ${forbidden}`);
  }
}
if (sharedCoreJoinRules.includes("SharedCore(store:")) {
  throw new Error("P4-S9-3 helper must not construct-and-drop SharedCore");
}
for (const forbidden of ["setRoomJoinRule", "imagePack", "roomLeave", "roomJoin("]) {
  if (sharedCoreJoinRules.includes(forbidden)) {
    throw new Error(`P4-S9-3 helper must not wrap ${forbidden}`);
  }
}
if (sharedCoreImagePacks.includes("SharedCore(store:")) {
  throw new Error("P4-S9-4 helper must not construct-and-drop SharedCore");
}
if (sharedCoreImagePacks.includes("SharedCore.newWithSecretStore") || sharedCoreImagePacks.includes("newWithSecretStore")) {
  throw new Error("P4-S9-4 helper must not construct SharedCore");
}
for (const forbidden of ["laterSnapshot", "mdirectSnapshot", "roomNotesSnapshot", "setOwnDisplayName", "setOwnAvatar"]) {
  if (sharedCoreImagePacks.includes(forbidden)) {
    throw new Error(`P4-S9-4 helper must not wrap ${forbidden}`);
  }
}
if (sharedCoreLater.includes("SharedCore(store:")) {
  throw new Error("P4-S9-5 helper must not construct-and-drop SharedCore");
}
if (sharedCoreLater.includes("SharedCore.newWithSecretStore") || sharedCoreLater.includes("newWithSecretStore")) {
  throw new Error("P4-S9-5 helper must not construct SharedCore");
}
for (const forbidden of ["mdirectSnapshot", "roomNotesSnapshot", "setOwnDisplayName", "setOwnAvatar"]) {
  if (sharedCoreLater.includes(forbidden)) {
    throw new Error(`P4-S9-5 helper must not wrap ${forbidden}`);
  }
}
if (sharedCoreMDirect.includes("SharedCore(store:")) {
  throw new Error("P4-S9-6 helper must not construct-and-drop SharedCore");
}
if (sharedCoreMDirect.includes("SharedCore.newWithSecretStore") || sharedCoreMDirect.includes("newWithSecretStore")) {
  throw new Error("P4-S9-6 helper must not construct SharedCore");
}
for (const forbidden of ["roomNotesSnapshot", "setOwnDisplayName", "setOwnAvatar", "laterSnapshot"]) {
  if (sharedCoreMDirect.includes(forbidden)) {
    throw new Error(`P4-S9-6 helper must not wrap ${forbidden}`);
  }
}
if (sharedCoreRoomNotes.includes("SharedCore(store:")) {
  throw new Error("P4-S9-7 helper must not construct-and-drop SharedCore");
}
if (sharedCoreRoomNotes.includes("SharedCore.newWithSecretStore") || sharedCoreRoomNotes.includes("newWithSecretStore")) {
  throw new Error("P4-S9-7 helper must not construct SharedCore");
}
for (const forbidden of ["setOwnDisplayName", "setOwnAvatar", "mdirectSnapshot", "laterSnapshot"]) {
  if (sharedCoreRoomNotes.includes(forbidden)) {
    throw new Error(`P4-S9-7 helper must not wrap ${forbidden}`);
  }
}
if (sharedCoreOwnProfile.includes("SharedCore(store:")) {
  throw new Error("P4-S9-8 helper must not construct-and-drop SharedCore");
}
if (sharedCoreOwnProfile.includes("SharedCore.newWithSecretStore") || sharedCoreOwnProfile.includes("newWithSecretStore")) {
  throw new Error("P4-S9-8 helper must not construct SharedCore");
}
for (const forbidden of ["setRoomName", "setRoomTopic", "setRoomAvatar", "roomNotesSnapshot", "backupStatus"]) {
  if (sharedCoreOwnProfile.includes(forbidden)) {
    throw new Error(`P4-S9-8 helper must not wrap ${forbidden}`);
  }
}
if (sharedCoreRoomProfile.includes("SharedCore(store:")) {
  throw new Error("P4-S9-9 helper must not construct-and-drop SharedCore");
}
if (sharedCoreRoomProfile.includes("SharedCore.newWithSecretStore") || sharedCoreRoomProfile.includes("newWithSecretStore")) {
  throw new Error("P4-S9-9 helper must not construct SharedCore");
}
for (const forbidden of ["getRoomDirectoryVisibility", "setRoomDirectoryVisibility", "roomJoinRuleSnapshot", "setOwnDisplayName", "backupStatus"]) {
  if (sharedCoreRoomProfile.includes(forbidden)) {
    throw new Error(`P4-S9-9 helper must not wrap ${forbidden}`);
  }
}
if (sharedCoreDirectoryVisibility.includes("SharedCore(store:")) {
  throw new Error("P4-S9-10 helper must not construct-and-drop SharedCore");
}
if (sharedCoreDirectoryVisibility.includes("SharedCore.newWithSecretStore") || sharedCoreDirectoryVisibility.includes("newWithSecretStore")) {
  throw new Error("P4-S9-10 helper must not construct SharedCore");
}
for (const forbidden of ["roomDirectorySearch", "roomDirectoryProtocols", "roomDirectoryCancel", "setRoomName", "backupStatus"]) {
  if (sharedCoreDirectoryVisibility.includes(forbidden)) {
    throw new Error(`P4-S9-10 helper must not wrap ${forbidden}`);
  }
}
if (sharedCoreDirectorySearch.includes("SharedCore(store:")) {
  throw new Error("P4-S9-11 helper must not construct-and-drop SharedCore");
}
if (sharedCoreDirectorySearch.includes("SharedCore.newWithSecretStore") || sharedCoreDirectorySearch.includes("newWithSecretStore")) {
  throw new Error("P4-S9-11 helper must not construct SharedCore");
}
for (const forbidden of ["roomLeave", "roomJoin(", "setRoomName", "backupStatus", "getRoomDirectoryVisibility"]) {
  if (sharedCoreDirectorySearch.includes(forbidden)) {
    throw new Error(`P4-S9-11 helper must not wrap ${forbidden}`);
  }
}
if (sharedCoreRoomLeaveJoin.includes("SharedCore(store:")) {
  throw new Error("P4-S9-12 helper must not construct-and-drop SharedCore");
}
if (sharedCoreRoomLeaveJoin.includes("SharedCore.newWithSecretStore") || sharedCoreRoomLeaveJoin.includes("newWithSecretStore")) {
  throw new Error("P4-S9-12 helper must not construct SharedCore");
}
for (const forbidden of ["roomInvite", "roomKick", "roomBan", "roomUnban", "roomDirectorySearch", "backupStatus"]) {
  if (sharedCoreRoomLeaveJoin.includes(forbidden)) {
    throw new Error(`P4-S9-12 helper must not wrap ${forbidden}`);
  }
}
if (sharedCoreRoomModeration.includes("SharedCore(store:")) {
  throw new Error("P4-S9-13 helper must not construct-and-drop SharedCore");
}
if (sharedCoreRoomModeration.includes("SharedCore.newWithSecretStore") || sharedCoreRoomModeration.includes("newWithSecretStore")) {
  throw new Error("P4-S9-13 helper must not construct SharedCore");
}
for (const forbidden of ["setPowerLevel", "roomCreate", "roomMembersSnapshot", "roomLeave", "backupStatus"]) {
  if (sharedCoreRoomModeration.includes(forbidden)) {
    throw new Error(`P4-S9-13 helper must not wrap ${forbidden}`);
  }
}
if (sharedCoreRoomPowerLevels.includes("SharedCore(store:")) {
  throw new Error("P4-S9-14 helper must not construct-and-drop SharedCore");
}
if (sharedCoreRoomPowerLevels.includes("SharedCore.newWithSecretStore") || sharedCoreRoomPowerLevels.includes("newWithSecretStore")) {
  throw new Error("P4-S9-14 helper must not construct SharedCore");
}
for (const forbidden of ["roomCreate", "roomMembersSnapshot", "roomInvite", "roomLeave", "backupStatus"]) {
  if (sharedCoreRoomPowerLevels.includes(forbidden)) {
    throw new Error(`P4-S9-14 helper must not wrap ${forbidden}`);
  }
}
if (sharedCoreRoomCreate.includes("SharedCore(store:")) {
  throw new Error("P4-S9-15 helper must not construct-and-drop SharedCore");
}
if (sharedCoreRoomCreate.includes("SharedCore.newWithSecretStore") || sharedCoreRoomCreate.includes("newWithSecretStore")) {
  throw new Error("P4-S9-15 helper must not construct SharedCore");
}
for (const forbidden of ["roomMembersSnapshot", "roomSetPowerLevel", "roomInvite", "roomLeave", "backupStatus"]) {
  if (sharedCoreRoomCreate.includes(forbidden)) {
    throw new Error(`P4-S9-15 helper must not wrap ${forbidden}`);
  }
}
if (sharedCoreRoomMembersSnapshots.includes("SharedCore(store:")) {
  throw new Error("P4-S9-16 helper must not construct-and-drop SharedCore");
}
if (sharedCoreRoomMembersSnapshots.includes("SharedCore.newWithSecretStore") || sharedCoreRoomMembersSnapshots.includes("newWithSecretStore")) {
  throw new Error("P4-S9-16 helper must not construct SharedCore");
}
for (const forbidden of ["roomCreate", "roomSetPowerLevel", "spaceParentsSnapshot", "roomInvite", "backupStatus"]) {
  if (sharedCoreRoomMembersSnapshots.includes(forbidden)) {
    throw new Error(`P4-S9-16 helper must not wrap ${forbidden}`);
  }
}
if (sharedCoreSpaces.includes("SharedCore(store:")) {
  throw new Error("P4-S9-17 helper must not construct-and-drop SharedCore");
}
if (sharedCoreSpaces.includes("SharedCore.newWithSecretStore") || sharedCoreSpaces.includes("newWithSecretStore")) {
  throw new Error("P4-S9-17 helper must not construct SharedCore");
}
for (const forbidden of ["roomMembersSnapshot", "invitesAccept", "roomCreate", "backupStatus"]) {
  if (sharedCoreSpaces.includes(forbidden)) {
    throw new Error(`P4-S9-17 helper must not wrap ${forbidden}`);
  }
}
if (sharedCoreInviteActions.includes("SharedCore(store:")) {
  throw new Error("P4-S9-18 helper must not construct-and-drop SharedCore");
}
if (sharedCoreInviteActions.includes("SharedCore.newWithSecretStore") || sharedCoreInviteActions.includes("newWithSecretStore")) {
  throw new Error("P4-S9-18 helper must not construct SharedCore");
}
for (const forbidden of ["invitesSnapshot", "jumpLatest", "setReadState", "sendText", "backupStatus"]) {
  if (sharedCoreInviteActions.includes(forbidden)) {
    throw new Error(`P4-S9-18 helper must not wrap ${forbidden}`);
  }
}
if (sharedCoreTimelineReadState.includes("SharedCore(store:")) {
  throw new Error("P4-S9-19 helper must not construct-and-drop SharedCore");
}
if (sharedCoreTimelineReadState.includes("SharedCore.newWithSecretStore") || sharedCoreTimelineReadState.includes("newWithSecretStore")) {
  throw new Error("P4-S9-19 helper must not construct SharedCore");
}
for (const forbidden of ["invitesAccept", "reactionToggle", "reactionEnsure", "sendText", "backupStatus"]) {
  if (sharedCoreTimelineReadState.includes(forbidden)) {
    throw new Error(`P4-S9-19 helper must not wrap ${forbidden}`);
  }
}
if (sharedCoreTimelineReactions.includes("SharedCore(store:")) {
  throw new Error("P4-S9-20 helper must not construct-and-drop SharedCore");
}
if (sharedCoreTimelineReactions.includes("SharedCore.newWithSecretStore") || sharedCoreTimelineReactions.includes("newWithSecretStore")) {
  throw new Error("P4-S9-20 helper must not construct SharedCore");
}
for (const forbidden of ["timelineEventReadback", "composerSetReplyDraft", "sendText", "backupStatus"]) {
  if (sharedCoreTimelineReactions.includes(forbidden)) {
    throw new Error(`P4-S9-20 helper must not wrap ${forbidden}`);
  }
}
if (sharedCoreComposerReplyDraft.includes("SharedCore(store:")) {
  throw new Error("P4-S9-21 helper must not construct-and-drop SharedCore");
}
if (sharedCoreComposerReplyDraft.includes("SharedCore.newWithSecretStore") || sharedCoreComposerReplyDraft.includes("newWithSecretStore")) {
  throw new Error("P4-S9-21 helper must not construct SharedCore");
}
for (const forbidden of ["reactionEnsure", "sendText", "backupStatus"]) {
  if (sharedCoreComposerReplyDraft.includes(forbidden)) {
    throw new Error(`P4-S9-21 helper must not wrap ${forbidden}`);
  }
}
if (sharedCoreSendText.includes("SharedCore(store:")) {
  throw new Error("P4-S9-22 helper must not construct-and-drop SharedCore");
}
if (sharedCoreSendText.includes("SharedCore.newWithSecretStore") || sharedCoreSendText.includes("newWithSecretStore")) {
  throw new Error("P4-S9-22 helper must not construct SharedCore");
}
for (const forbidden of ["composerSetReplyDraft", "sendSticker", "sendPoll", "editMessage", "pollRespond", "backupStatus"]) {
  if (sharedCoreSendText.includes(forbidden)) {
    throw new Error(`P4-S9-22 helper must not wrap ${forbidden}`);
  }
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
const timelineEventReadbackDto = udl.match(/dictionary TimelineEventReadbackDto \{([\s\S]*?)\};/);
if (!timelineEventReadbackDto) throw new Error("missing TimelineEventReadbackDto");
if (/\bpassword\b/.test(timelineEventReadbackDto[1]) || /\btoken\b/.test(timelineEventReadbackDto[1])) {
  throw new Error("TimelineEventReadbackDto must not carry password or token fields");
}
const timelineReadStateDto = udl.match(/dictionary TimelineReadStateDto \{([\s\S]*?)\};/);
if (!timelineReadStateDto) throw new Error("missing TimelineReadStateDto");
if (/\bpassword\b/.test(timelineReadStateDto[1]) || /\btoken\b/.test(timelineReadStateDto[1])) {
  throw new Error("TimelineReadStateDto must not carry password or token fields");
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
const joinRuleDto = udl.match(/dictionary RoomJoinRuleSnapshotDto \{([\s\S]*?)\};/);
if (!joinRuleDto) throw new Error("missing RoomJoinRuleSnapshotDto");
if (/\bpassword\b/.test(joinRuleDto[1]) || /\btoken\b/.test(joinRuleDto[1])) {
  throw new Error("RoomJoinRuleSnapshotDto must not carry password or token fields");
}
const imagePackDto = udl.match(/dictionary ImagePackDto \{([\s\S]*?)\};/);
if (!imagePackDto) throw new Error("missing ImagePackDto");
if (/\bpassword\b/.test(imagePackDto[1]) || /\btoken\b/.test(imagePackDto[1]) || /\bbytes\b/.test(imagePackDto[1])) {
  throw new Error("ImagePackDto must not carry password, token, or bytes fields");
}
const laterItemDto = udl.match(/dictionary LaterItemDto \{([\s\S]*?)\};/);
if (!laterItemDto) throw new Error("missing LaterItemDto");
if (/\bpassword\b/.test(laterItemDto[1]) || /\btoken\b/.test(laterItemDto[1]) || /\bbytes\b/.test(laterItemDto[1])) {
  throw new Error("LaterItemDto must not carry password, token, or bytes fields");
}
const mdirectSnapshotDto = udl.match(/dictionary MDirectSnapshotDto \{([\s\S]*?)\};/);
if (!mdirectSnapshotDto) throw new Error("missing MDirectSnapshotDto");
if (/\bpassword\b/.test(mdirectSnapshotDto[1]) || /\btoken\b/.test(mdirectSnapshotDto[1]) || /\bbytes\b/.test(mdirectSnapshotDto[1])) {
  throw new Error("MDirectSnapshotDto must not carry password, token, or bytes fields");
}
const roomNoteItemDto = udl.match(/dictionary RoomNoteItemDto \{([\s\S]*?)\};/);
if (!roomNoteItemDto) throw new Error("missing RoomNoteItemDto");
if (/\bpassword\b/.test(roomNoteItemDto[1]) || /\btoken\b/.test(roomNoteItemDto[1]) || /\bbytes\b/.test(roomNoteItemDto[1])) {
  throw new Error("RoomNoteItemDto must not carry password, token, or bytes fields");
}
const ownProfileWriteDto = udl.match(/dictionary OwnProfileWriteDto \{([\s\S]*?)\};/);
if (!ownProfileWriteDto) throw new Error("missing OwnProfileWriteDto");
if (/\bpassword\b/.test(ownProfileWriteDto[1]) || /\btoken\b/.test(ownProfileWriteDto[1]) || /\bbytes\b/.test(ownProfileWriteDto[1]) || /\bdisplay_name\b/.test(ownProfileWriteDto[1]) || /\bmxc\b/.test(ownProfileWriteDto[1])) {
  throw new Error("OwnProfileWriteDto must not carry password, token, bytes, display_name, or mxc fields");
}
const roomProfileWriteDto = udl.match(/dictionary RoomProfileWriteDto \{([\s\S]*?)\};/);
if (!roomProfileWriteDto) throw new Error("missing RoomProfileWriteDto");
if (/\bpassword\b/.test(roomProfileWriteDto[1]) || /\btoken\b/.test(roomProfileWriteDto[1]) || /\bbytes\b/.test(roomProfileWriteDto[1]) || /\broom_id\b/.test(roomProfileWriteDto[1]) || /\bname\b/.test(roomProfileWriteDto[1]) || /\btopic\b/.test(roomProfileWriteDto[1]) || /\bmxc\b/.test(roomProfileWriteDto[1])) {
  throw new Error("RoomProfileWriteDto must not carry password, token, bytes, room_id, name, topic, or mxc fields");
}
const roomDirectoryVisibilityDto = udl.match(/dictionary RoomDirectoryVisibilityDto \{([\s\S]*?)\};/);
if (!roomDirectoryVisibilityDto) throw new Error("missing RoomDirectoryVisibilityDto");
if (/\bpassword\b/.test(roomDirectoryVisibilityDto[1]) || /\btoken\b/.test(roomDirectoryVisibilityDto[1]) || /\bbytes\b/.test(roomDirectoryVisibilityDto[1]) || /\bmxc\b/.test(roomDirectoryVisibilityDto[1])) {
  throw new Error("RoomDirectoryVisibilityDto must not carry password, token, bytes, or mxc fields");
}
const roomDirectoryVisibilityWriteDto = udl.match(/dictionary RoomDirectoryVisibilityWriteDto \{([\s\S]*?)\};/);
if (!roomDirectoryVisibilityWriteDto) throw new Error("missing RoomDirectoryVisibilityWriteDto");
if (/\bpassword\b/.test(roomDirectoryVisibilityWriteDto[1]) || /\btoken\b/.test(roomDirectoryVisibilityWriteDto[1]) || /\bbytes\b/.test(roomDirectoryVisibilityWriteDto[1]) || /\bmxc\b/.test(roomDirectoryVisibilityWriteDto[1])) {
  throw new Error("RoomDirectoryVisibilityWriteDto must not carry password, token, bytes, or mxc fields");
}
const roomDirectorySearchDto = udl.match(/dictionary RoomDirectorySearchDto \{([\s\S]*?)\};/);
if (!roomDirectorySearchDto) throw new Error("missing RoomDirectorySearchDto");
if (/\bpassword\b/.test(roomDirectorySearchDto[1]) || /\btoken\b/.test(roomDirectorySearchDto[1]) || /\bbytes\b/.test(roomDirectorySearchDto[1])) {
  throw new Error("RoomDirectorySearchDto must not carry password, token, or bytes fields");
}
const roomDirectoryHitDto = udl.match(/dictionary RoomDirectoryHitDto \{([\s\S]*?)\};/);
if (!roomDirectoryHitDto) throw new Error("missing RoomDirectoryHitDto");
if (/\bpassword\b/.test(roomDirectoryHitDto[1]) || /\btoken\b/.test(roomDirectoryHitDto[1]) || /\bbytes\b/.test(roomDirectoryHitDto[1])) {
  throw new Error("RoomDirectoryHitDto must not carry password, token, or bytes fields");
}
const roomDirectoryProtocolsDto = udl.match(/dictionary RoomDirectoryProtocolsDto \{([\s\S]*?)\};/);
if (!roomDirectoryProtocolsDto) throw new Error("missing RoomDirectoryProtocolsDto");
if (/\bpassword\b/.test(roomDirectoryProtocolsDto[1]) || /\btoken\b/.test(roomDirectoryProtocolsDto[1]) || /\bbytes\b/.test(roomDirectoryProtocolsDto[1])) {
  throw new Error("RoomDirectoryProtocolsDto must not carry password, token, or bytes fields");
}
const roomMembershipWriteDto = udl.match(/dictionary RoomMembershipWriteDto \{([\s\S]*?)\};/);
if (!roomMembershipWriteDto) throw new Error("missing RoomMembershipWriteDto");
if (/\bpassword\b/.test(roomMembershipWriteDto[1]) || /\btoken\b/.test(roomMembershipWriteDto[1]) || /\bbytes\b/.test(roomMembershipWriteDto[1]) || /\broom_id\b/.test(roomMembershipWriteDto[1]) || /\balias\b/.test(roomMembershipWriteDto[1]) || /\bvia\b/.test(roomMembershipWriteDto[1])) {
  throw new Error("RoomMembershipWriteDto must not carry password, token, bytes, room_id, alias, or via fields");
}
const roomModerationWriteDto = udl.match(/dictionary RoomModerationWriteDto \{([\s\S]*?)\};/);
if (!roomModerationWriteDto) throw new Error("missing RoomModerationWriteDto");
if (/\bpassword\b/.test(roomModerationWriteDto[1]) || /\btoken\b/.test(roomModerationWriteDto[1]) || /\bbytes\b/.test(roomModerationWriteDto[1]) || /\broom_id\b/.test(roomModerationWriteDto[1]) || /\buser_id\b/.test(roomModerationWriteDto[1]) || /\breason\b/.test(roomModerationWriteDto[1])) {
  throw new Error("RoomModerationWriteDto must not carry password, token, bytes, room_id, user_id, or reason fields");
}
const roomPowerLevelWriteDto = udl.match(/dictionary RoomPowerLevelWriteDto \{([\s\S]*?)\};/);
if (!roomPowerLevelWriteDto) throw new Error("missing RoomPowerLevelWriteDto");
if (/\bpassword\b/.test(roomPowerLevelWriteDto[1]) || /\btoken\b/.test(roomPowerLevelWriteDto[1]) || /\bbytes\b/.test(roomPowerLevelWriteDto[1]) || /\broom_id\b/.test(roomPowerLevelWriteDto[1]) || /\buser_id\b/.test(roomPowerLevelWriteDto[1]) || /\bpower_level\b/.test(roomPowerLevelWriteDto[1]) || /\bcontent\b/.test(roomPowerLevelWriteDto[1])) {
  throw new Error("RoomPowerLevelWriteDto must not carry password, token, bytes, room_id, user_id, power_level, or content fields");
}
const roomCreateRequestDto = udl.match(/dictionary RoomCreateRequestDto \{([\s\S]*?)\};/);
if (!roomCreateRequestDto) throw new Error("missing RoomCreateRequestDto");
if (/\bpassword\b/.test(roomCreateRequestDto[1]) || /\btoken\b/.test(roomCreateRequestDto[1]) || /\bbytes\b/.test(roomCreateRequestDto[1]) || /\bpath\b/.test(roomCreateRequestDto[1]) || /\bpassphrase\b/.test(roomCreateRequestDto[1]) || /\bcreation_content\b/.test(roomCreateRequestDto[1]) || /\bpower_level_content_override\b/.test(roomCreateRequestDto[1])) {
  throw new Error("RoomCreateRequestDto must not carry password, token, bytes, path, passphrase, creation_content, or power_level_content_override fields");
}
const roomCreateDto = udl.match(/dictionary RoomCreateDto \{([\s\S]*?)\};/);
if (!roomCreateDto) throw new Error("missing RoomCreateDto");
if (/\bpassword\b/.test(roomCreateDto[1]) || /\btoken\b/.test(roomCreateDto[1]) || /\bbytes\b/.test(roomCreateDto[1]) || /\bname\b/.test(roomCreateDto[1]) || /\btopic\b/.test(roomCreateDto[1]) || /\balias\b/.test(roomCreateDto[1]) || /\binvite\b/.test(roomCreateDto[1])) {
  throw new Error("RoomCreateDto must not carry password, token, bytes, name, topic, alias, or invite fields");
}
const roomMembersSnapshotDto = udl.match(/dictionary RoomMembersSnapshotDto \{([\s\S]*?)\};/);
if (!roomMembersSnapshotDto) throw new Error("missing RoomMembersSnapshotDto");
if (/\bpassword\b/.test(roomMembersSnapshotDto[1]) || /\btoken\b/.test(roomMembersSnapshotDto[1]) || /\bbytes\b/.test(roomMembersSnapshotDto[1]) || /\bpath\b/.test(roomMembersSnapshotDto[1]) || /\bpassphrase\b/.test(roomMembersSnapshotDto[1])) {
  throw new Error("RoomMembersSnapshotDto must not carry password, token, bytes, path, or passphrase fields");
}
const roomMemberDto = udl.match(/dictionary RoomMemberDto \{([\s\S]*?)\};/);
if (!roomMemberDto) throw new Error("missing RoomMemberDto");
if (/\bpassword\b/.test(roomMemberDto[1]) || /\btoken\b/.test(roomMemberDto[1]) || /\bbytes\b/.test(roomMemberDto[1]) || /\bpath\b/.test(roomMemberDto[1]) || /\bpassphrase\b/.test(roomMemberDto[1])) {
  throw new Error("RoomMemberDto must not carry password, token, bytes, path, or passphrase fields");
}
const roomPowerLevelsSnapshotDto = udl.match(/dictionary RoomPowerLevelsSnapshotDto \{([\s\S]*?)\};/);
if (!roomPowerLevelsSnapshotDto) throw new Error("missing RoomPowerLevelsSnapshotDto");
if (/\bpassword\b/.test(roomPowerLevelsSnapshotDto[1]) || /\btoken\b/.test(roomPowerLevelsSnapshotDto[1]) || /\bbytes\b/.test(roomPowerLevelsSnapshotDto[1]) || /\bpath\b/.test(roomPowerLevelsSnapshotDto[1]) || /\bpassphrase\b/.test(roomPowerLevelsSnapshotDto[1])) {
  throw new Error("RoomPowerLevelsSnapshotDto must not carry password, token, bytes, path, or passphrase fields");
}
const roomCreatorsSnapshotDto = udl.match(/dictionary RoomCreatorsSnapshotDto \{([\s\S]*?)\};/);
if (!roomCreatorsSnapshotDto) throw new Error("missing RoomCreatorsSnapshotDto");
if (/\bpassword\b/.test(roomCreatorsSnapshotDto[1]) || /\btoken\b/.test(roomCreatorsSnapshotDto[1]) || /\bbytes\b/.test(roomCreatorsSnapshotDto[1]) || /\bpath\b/.test(roomCreatorsSnapshotDto[1]) || /\bpassphrase\b/.test(roomCreatorsSnapshotDto[1])) {
  throw new Error("RoomCreatorsSnapshotDto must not carry password, token, bytes, path, or passphrase fields");
}
const roomPowerLevelTagsSnapshotDto = udl.match(/dictionary RoomPowerLevelTagsSnapshotDto \{([\s\S]*?)\};/);
if (!roomPowerLevelTagsSnapshotDto) throw new Error("missing RoomPowerLevelTagsSnapshotDto");
if (/\bpassword\b/.test(roomPowerLevelTagsSnapshotDto[1]) || /\btoken\b/.test(roomPowerLevelTagsSnapshotDto[1]) || /\bbytes\b/.test(roomPowerLevelTagsSnapshotDto[1]) || /\bpath\b/.test(roomPowerLevelTagsSnapshotDto[1]) || /\bpassphrase\b/.test(roomPowerLevelTagsSnapshotDto[1])) {
  throw new Error("RoomPowerLevelTagsSnapshotDto must not carry password, token, bytes, path, or passphrase fields");
}
for (const [name, dto] of [
  ["SpaceParentsSnapshotDto", udl.match(/dictionary SpaceParentsSnapshotDto \{([\s\S]*?)\};/)],
  ["SpaceHierarchySnapshotDto", udl.match(/dictionary SpaceHierarchySnapshotDto \{([\s\S]*?)\};/)],
  ["SpaceChildrenSnapshotDto", udl.match(/dictionary SpaceChildrenSnapshotDto \{([\s\S]*?)\};/)],
  ["SpaceChildMutationDto", udl.match(/dictionary SpaceChildMutationDto \{([\s\S]*?)\};/)],
  ["RestrictedJoinReparentDto", udl.match(/dictionary RestrictedJoinReparentDto \{([\s\S]*?)\};/)],
  ["SpaceHierarchyRoomDto", udl.match(/dictionary SpaceHierarchyRoomDto \{([\s\S]*?)\};/)],
  ["SpaceChildEdgeDto", udl.match(/dictionary SpaceChildEdgeDto \{([\s\S]*?)\};/)],
]) {
  if (!dto) throw new Error(`missing ${name}`);
  if (/\bpassword\b/.test(dto[1]) || /\btoken\b/.test(dto[1]) || /\bbytes\b/.test(dto[1]) || /\bpath\b/.test(dto[1]) || /\bpassphrase\b/.test(dto[1])) {
    throw new Error(`${name} must not carry password, token, bytes, path, or passphrase fields`);
  }
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
