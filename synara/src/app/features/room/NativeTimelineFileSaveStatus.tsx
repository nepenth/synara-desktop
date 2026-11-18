import React, { useEffect, useState } from 'react';
import { Box, Icon, IconButton, Icons, ProgressBar, Text } from 'folds';
import { revealSavedMatrixMediaFile } from '../../matrix/media';
import {
  dismissNativeTimelineFileSave,
  nativeTimelineFileSavedMessage,
  nativeTimelineFileSavePercent,
  type NativeTimelineFileSaveState,
} from './nativeTimelineFileSaveState';
import * as htmlCss from './nativeTimelineHtml.css';

export const REVEAL_DOWNLOAD_FAILED = 'Synara could not open your Downloads folder.';

/** Show-in-folder for a saved file; `onError` gets the copy for a failed reveal. */
export const revealNativeTimelineSavedFile = (
  filename: string,
  onError: (message: string) => void
): void => {
  void revealSavedMatrixMediaFile(filename).catch(() => onError(REVEAL_DOWNLOAD_FAILED));
};

/** Percentage and bar beside a file while it downloads, then "Finishing…". */
export function NativeTimelineFileSaveProgress({
  state,
  label,
}: {
  state: NativeTimelineFileSaveState | undefined;
  label: string;
}) {
  if (state?.kind === 'finishing') {
    return (
      <Text as="span" size="T200" className={htmlCss.Metadata}>
        Finishing…
      </Text>
    );
  }
  if (state?.kind !== 'downloading') return null;
  const percent = state.total ? nativeTimelineFileSavePercent(state.received, state.total) : null;
  return (
    <Box
      as="span"
      alignItems="Center"
      gap="200"
      data-native-timeline-file-save-progress={percent === null ? 'indeterminate' : 'determinate'}
    >
      <Text as="span" size="T200" className={htmlCss.Metadata} aria-hidden>
        {percent === null ? 'Downloading…' : `${percent}%`}
      </Text>
      {percent === null ? (
        // No max: folds draws its indeterminate packet.
        <ProgressBar
          className={htmlCss.FileSaveProgressBar}
          size="300"
          variant="Secondary"
          value={0}
          aria-valuenow={undefined}
          aria-valuemin={undefined}
          aria-label={`Downloading ${label}`}
        />
      ) : (
        <ProgressBar
          className={htmlCss.FileSaveProgressBar}
          size="300"
          variant="Secondary"
          min={0}
          max={100}
          value={percent}
          aria-label={`Downloading ${label}`}
          aria-valuetext={`${percent}%`}
        />
      )}
    </Box>
  );
}

/**
 * The line under a file once its save is over: where it went, with Show in
 * folder, or why it failed, with Retry. Each dismisses on its own.
 */
export function NativeTimelineFileSaveResult({
  handleId,
  state,
  onRetry,
}: {
  handleId: string;
  state: NativeTimelineFileSaveState | undefined;
  onRetry: () => void;
}) {
  const [revealError, setRevealError] = useState<string>();
  const savedName = state?.kind === 'done' ? state.filename : undefined;
  useEffect(() => setRevealError(undefined), [savedName]);
  if (state?.kind !== 'done' && state?.kind !== 'failed') return null;
  const dismiss = (
    <IconButton
      size="300"
      radii="300"
      variant="SurfaceVariant"
      fill="None"
      aria-label="Dismiss"
      onClick={() => dismissNativeTimelineFileSave(handleId)}
    >
      <Icon src={Icons.Cross} size="50" />
    </IconButton>
  );
  if (state.kind === 'failed') {
    return (
      <Box
        className={htmlCss.FileSaveStatus}
        alignItems="Center"
        gap="200"
        wrap="Wrap"
        role="alert"
        data-native-timeline-file-save="failed"
      >
        <Text as="span" size="T200" className={htmlCss.FileSaveFailed}>
          {state.message}
        </Text>
        {state.retryable ? (
          <button type="button" className={htmlCss.FileDownload} onClick={onRetry}>
            <Text as="span" size="T200">
              Retry
            </Text>
          </button>
        ) : null}
        {dismiss}
      </Box>
    );
  }
  return (
    <Box
      className={htmlCss.FileSaveStatus}
      alignItems="Center"
      gap="200"
      wrap="Wrap"
      role="status"
      data-native-timeline-file-save="done"
    >
      <Text as="span" size="T200" className={htmlCss.Metadata}>
        {nativeTimelineFileSavedMessage(state.filename)}
      </Text>
      <button
        type="button"
        className={htmlCss.FileDownload}
        onClick={() => revealNativeTimelineSavedFile(state.filename, setRevealError)}
      >
        <Text as="span" size="T200">
          Show in folder
        </Text>
      </button>
      {dismiss}
      {revealError ? (
        <Text as="span" size="T200" className={htmlCss.FileSaveFailed} role="alert">
          {revealError}
        </Text>
      ) : null}
    </Box>
  );
}
