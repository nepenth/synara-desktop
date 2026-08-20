import { spawn } from 'node:child_process';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const tests = [
  'src/app/components/__tests__/roomAvatarSourceGuard.test.ts',
  'src/app/components/__tests__/joinRulesSwitcherSourceGuard.test.ts',
  'src/app/config/__tests__/foundationFeatures.test.ts',
  'src/app/__tests__/cs-api-security.test.ts',
  'src/app/utils/__tests__/remoteContent.test.ts',
  'src/app/contracts/__tests__/contractSchemas.test.ts',
  'src/app/hooks/__tests__/useVirtualPaginator.test.ts',
  'src/app/hooks/__tests__/useAsyncSearchRenderSafety.test.ts',
  'src/app/hooks/__tests__/nativeRoomMembersOwner.test.ts',
  'src/app/hooks/__tests__/nativeRoomPowerReadOwner.test.ts',
  'src/app/hooks/__tests__/roomDirectoryVisibilitySourceGuard.test.ts',
  'src/app/hooks/__tests__/roomMembersWiring.test.ts',
  'src/app/plugins/__tests__/via-servers.test.ts',
  'src/app/utils/__tests__/gifProvider.test.ts',
  'src/app/utils/__tests__/later.test.ts',
  'src/app/utils/__tests__/roomNotes.test.ts',
  'src/app/utils/__tests__/roomIcon.test.ts',
  'src/app/utils/__tests__/common.test.ts',
  'src/app/utils/__tests__/room.test.ts',
  'src/app/utils/__tests__/notifications.test.ts',
  'src/app/utils/__tests__/foundationDiagnostics.test.ts',
  'src/app/utils/__tests__/clientDiagnostics.test.ts',
  'src/app/utils/__tests__/diagnosticsReport.test.ts',
  'src/app/utils/__tests__/boundedLru.test.ts',
  'src/app/notifications/__tests__/badgeSummary.test.ts',
  'src/app/notifications/__tests__/systemNotification.test.ts',
  'src/app/routes/__tests__/synaraRoutes.test.ts',
  'src/app/utils/__tests__/agentApprovals.test.ts',
  'src/app/utils/__tests__/hermes.test.ts',
  'src/app/agents/__tests__/agentActions.test.ts',
  'src/app/utils/__tests__/drafts.test.ts',
  'src/app/utils/__tests__/polls.test.ts',
  'src/app/utils/__tests__/dom.test.ts',
  'src/app/utils/__tests__/appLinks.test.ts',
  'src/app/utils/__tests__/messageSearchFilters.test.ts',
  'src/app/utils/__tests__/matrix.test.ts',
  'src/app/utils/__tests__/themeAccent.test.ts',
  'src/app/utils/__tests__/themeBase.test.ts',
  'src/app/utils/__tests__/syncLifecycle.test.ts',
  'src/app/utils/__tests__/syncSplashRecovery.test.ts',
  'src/app/utils/__tests__/timelinePagination.test.ts',
  'src/app/utils/__tests__/timelineLinks.test.ts',
  'src/app/utils/__tests__/timelineNavigation.test.ts',
  'src/app/utils/__tests__/timelineOpening.test.ts',
  'src/app/pages/client/__tests__/syncStatusCopy.test.ts',
  'src/app/pages/__tests__/pathUtils.test.ts',
  'src/app/pages/__tests__/leftoverBrowserClientGuard.test.ts',
  'src/app/utils/__tests__/timelineVirtualization.test.ts',
  'src/app/utils/__tests__/desktop.test.ts',
  'src/app/utils/__tests__/desktopUpdater.test.ts',
  'src/app/platform/__tests__/platform.test.ts',
  'src/app/platform/__tests__/agentActions.test.ts',
  'src/app/matrix/__tests__/media.test.ts',
  'src/app/matrix/__tests__/matrixLocalStores.test.ts',
  'src/app/features/matrix-ipc/__tests__/matrixIpc.test.ts',
  'src/app/features/matrix-ipc/__tests__/matrixIpcContract.test.ts',
  'src/app/features/matrix-dto/__tests__/matrixDto.test.ts',
  'src/app/features/matrix-presence/__tests__/nativePresence.test.ts',
  'src/app/pages/client/explore/__tests__/nativeRoomDirectoryOwner.test.ts',
  'src/app/pages/client/explore/__tests__/roomDirectorySourceGuard.test.ts',
  'src/app/features/lobby/__tests__/nativeSpaceHierarchyOwner.test.ts',
  'src/app/features/room/__tests__/nativeSendText.test.ts',
  'src/app/features/room/__tests__/nativeSendAttachmentOwner.test.ts',
  'src/app/features/room/__tests__/nativeSendStickerOwner.test.ts',
  'src/app/features/room/__tests__/nativeSendGifOwner.test.ts',
  'src/app/features/room/__tests__/nativeMDirectOwner.test.ts',
  'src/app/features/room/__tests__/nativePollOwner.test.ts',
  'src/app/features/room/__tests__/nativeImagePackOwner.test.ts',
  'src/app/state/__tests__/nativeMediaUploadOwner.test.ts',
  'src/app/features/settings/account/__tests__/nativeProfileOwner.test.ts',
  'src/app/features/settings/__tests__/settingsSurfaceGuard.test.ts',
  'src/app/features/common-settings/general/__tests__/nativeRoomProfileOwner.test.ts',
  'src/app/features/native-client/__tests__/nativeClientFacade.test.ts',
  'src/app/features/common-settings/general/__tests__/nativeRoomJoinRuleOwner.test.ts',
  'src/app/features/common-settings/general/__tests__/roomPublishSourceGuard.test.ts',
  'src/app/components/__tests__/nativeRoomLeaveOwner.test.ts',
  'src/app/components/__tests__/nativeRoomFavoriteOwner.test.ts',
  'src/app/pages/client/home/__tests__/homeRoomList.test.ts',
  'src/app/components/__tests__/nativeRoomJoinOwner.test.ts',
  'src/app/components/__tests__/nativeRoomCreateOwner.test.ts',
  'src/app/components/__tests__/nativeRoomModerationOwner.test.ts',
  'src/app/features/room/__tests__/nativeEditMessageOwner.test.ts',
  'src/app/features/room/__tests__/nativeReactionOwner.test.ts',
  'src/app/features/room/__tests__/nativeTimelineActions.test.ts',
  'src/app/features/room/__tests__/nativeTimelineCodeHighlight.test.ts',
  'src/app/features/room/__tests__/nativeTimelinePresenterActions.test.ts',
  'src/app/features/room/__tests__/nativeTimelineViewportPolicy.test.ts',
  'src/app/features/room/__tests__/nativeTimelineViewDelta.test.ts',
  'src/app/features/verification/__tests__/nativeVerification.test.ts',
  'src/app/features/cross-signing/__tests__/nativeCrossSigning.test.ts',
  'src/app/features/backup/__tests__/nativeBackup.test.ts',
  'src/app/features/secret-storage/__tests__/nativeSecretStorage.test.ts',
  'src/app/features/room-keys/__tests__/nativeRoomKeys.test.ts',
  'src/app/state/__tests__/initMatrix.test.ts',

  'src/app/state/__tests__/sessionBootstrap.test.ts',
  'src/app/state/__tests__/sessionPersistence.test.ts',
  'src/app/state/__tests__/sessions.test.ts',
  'src/app/state/__tests__/clearLoginData.test.ts',
  'src/app/state/__tests__/performLogout.test.ts',
  'src/app/state/__tests__/settings.test.ts',
  'src/app/state/room-list/__tests__/roomActivity.test.ts',
  'src/app/state/room/__tests__/roomToUnread.test.ts',
  'src/app/state/room/__tests__/roomToParents.test.ts',
  'src/app/state/__tests__/mDirectList.test.ts',
  'src/app/state/__tests__/typingMembers.test.ts',
  'src/app/components/editor/__tests__/richText.test.ts',
  'src/app/pages/auth/login/__tests__/nativeLoginFlows.test.ts',
  'src/app/pages/auth/reset-password/__tests__/nativePasswordReset.test.ts',
  'src/app/pages/auth/register/__tests__/nativeRegister.test.ts',
  'src/app/pages/auth/login/__tests__/tokenLoginAbsence.test.ts',
  'src/app/pages/auth/login/__tests__/loginUiaStageAbsence.test.ts',
  'src/app/pages/auth/login/__tests__/desktopPasswordLoginNativeOnly.test.ts',
  'src/app/pages/auth/login/__tests__/nativeLoginBootstrap.test.ts',
];

const outdir = await mkdtemp(join(tmpdir(), 'synara-modernization-tests-'));
const projectRoot = fileURLToPath(new URL('..', import.meta.url));

const run = (command, args) =>
  new Promise((resolve) => {
    const child = spawn(command, args, {
      cwd: projectRoot,
      stdio: 'inherit',
    });
    child.on('exit', resolve);
  });

try {
  const esbuildBin = join(
    projectRoot,
    'node_modules',
    '.bin',
    process.platform === 'win32' ? 'esbuild.cmd' : 'esbuild'
  );
  const buildExitCode = await run(esbuildBin, [
    ...tests,
    '--bundle',
    '--platform=node',
    '--format=cjs',
    '--log-level=silent',
    `--outdir=${outdir}`,
  ]);
  if (buildExitCode !== 0) {
    process.exit(buildExitCode ?? 1);
  }

  const testFiles = tests.map((testPath) =>
    join(outdir, testPath.replace(/^src\/app\//, '').replace(/\.ts$/, '.js'))
  );

  const exitCode = await run(process.execPath, ['--test', ...testFiles]);

  if (exitCode !== 0) {
    process.exit(exitCode ?? 1);
  }
} finally {
  await rm(outdir, { recursive: true, force: true });
}
