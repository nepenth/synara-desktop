import React from 'react';
import { Box, Button, Spinner, Text, color, config } from 'folds';
import type { TimelineHistoryOverlayKind } from '../../utils/timelinePagination';
import * as htmlCss from './nativeTimelineHtml.css';
import * as depthCss from '../../styles/Depth.css';

type NativeTimelineHistoryStatusProps = {
  edge: 'backward' | 'forward';
  kind: TimelineHistoryOverlayKind;
  errorMessage?: string;
  visibleDateLabel?: string;
  onRetry?: () => void;
  onLoadMore?: () => void;
};

const copyForEdge = (edge: 'backward' | 'forward') =>
  edge === 'backward'
    ? {
        loadingLabel: 'Loading older messages',
        loadingText: 'Loading older messages…',
        errorTitle: 'Could not load older messages',
        loadMore: 'Load older messages',
      }
    : {
        loadingLabel: 'Loading newer messages',
        loadingText: 'Loading newer messages…',
        errorTitle: 'Could not load newer messages',
        loadMore: 'Load newer messages',
      };

export function NativeTimelineHistoryStatus({
  edge,
  kind,
  errorMessage,
  visibleDateLabel,
  onRetry,
  onLoadMore,
}: NativeTimelineHistoryStatusProps) {
  const copy = copyForEdge(edge);
  const showDate = Boolean(visibleDateLabel) && kind === 'hidden';
  if (kind === 'hidden' && !showDate) return null;

  return (
    <Box
      className={
        edge === 'backward'
          ? htmlCss.HistoryStatusOverlayBackward
          : htmlCss.HistoryStatusOverlayForward
      }
      justifyContent="Center"
      style={{ pointerEvents: kind === 'hidden' ? 'none' : undefined }}
    >
      {kind === 'loading' ? (
        <Box
          className={`${htmlCss.HistoryStatusCard} ${depthCss.floatingSurface}`}
          alignItems="Center"
          gap="200"
          role="status"
          aria-live="polite"
          aria-label={copy.loadingLabel}
          data-timeline-history-status="loading"
          data-timeline-history-edge={edge}
        >
          <Spinner size="200" aria-label={copy.loadingLabel} />
          <Text size="T300">{copy.loadingText}</Text>
        </Box>
      ) : null}
      {kind === 'error' ? (
        <Box
          className={`${htmlCss.HistoryStatusCard} ${depthCss.floatingSurface}`}
          alignItems="Center"
          gap="200"
          role="alert"
          data-timeline-history-status="error"
          data-timeline-history-edge={edge}
        >
          <Box direction="Column" gap="100" style={{ minWidth: 0 }}>
            <Text size="T300" style={{ color: color.Critical.Main }}>
              {copy.errorTitle}
            </Text>
            {errorMessage ? (
              <Text size="T200" style={{ color: color.Critical.Main }}>
                {errorMessage}
              </Text>
            ) : null}
          </Box>
          {onRetry ? (
            <Button variant="Critical" fill="Soft" size="300" onClick={onRetry}>
              <Text size="B300">Retry</Text>
            </Button>
          ) : null}
        </Box>
      ) : null}
      {kind === 'load_more' && onLoadMore ? (
        <Button
          variant="Secondary"
          fill="Soft"
          radii="Pill"
          outlined
          size="300"
          onClick={onLoadMore}
        >
          <Text size="B300">{copy.loadMore}</Text>
        </Button>
      ) : null}
      {showDate ? (
        <Box
          className={`${htmlCss.HistoryStatusCard} ${depthCss.floatingSurface}`}
          style={{ padding: `${config.space.S100} ${config.space.S300}` }}
          data-timeline-history-status="date"
        >
          <Text size="T200">{visibleDateLabel}</Text>
        </Box>
      ) : null}
    </Box>
  );
}
