import React from 'react';
import { Button, Spinner, Text, color, config } from 'folds';
import type { TimelineHistoryOverlayKind } from '../../utils/timelinePagination';
import * as htmlCss from './nativeTimelineHtml.css';

type NativeTimelineHistoryStatusProps = {
  edge: 'backward' | 'forward';
  kind: TimelineHistoryOverlayKind;
  errorMessage?: string;
  visibleDateLabel?: string;
  reserveRail?: boolean;
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
  reserveRail,
  onRetry,
  onLoadMore,
}: NativeTimelineHistoryStatusProps) {
  const copy = copyForEdge(edge);
  const showDate = Boolean(visibleDateLabel) && kind === 'hidden';
  if (kind === 'hidden' && !showDate) return null;

  return (
    <div
      className={
        edge === 'backward'
          ? htmlCss.HistoryStatusOverlayBackward
          : htmlCss.HistoryStatusOverlayForward
      }
      style={{
        pointerEvents: kind === 'hidden' ? 'none' : 'auto',
        ...(reserveRail ? undefined : { right: 0 }),
      }}
    >
      {kind === 'loading' ? (
        <div
          className={htmlCss.HistoryStatusCard}
          role="status"
          aria-live="polite"
          aria-label={copy.loadingLabel}
          data-timeline-history-status="loading"
          data-timeline-history-edge={edge}
        >
          <Spinner size="200" variant="Secondary" aria-label={copy.loadingLabel} />
          <Text size="T300">{copy.loadingText}</Text>
        </div>
      ) : null}
      {kind === 'error' ? (
        <div
          className={`${htmlCss.HistoryStatusCard} ${htmlCss.HistoryStatusCardError}`}
          role="alert"
          data-timeline-history-status="error"
          data-timeline-history-edge={edge}
        >
          <div style={{ minWidth: 0 }}>
            <Text size="T300" style={{ color: color.Critical.OnContainer }}>
              {copy.errorTitle}
            </Text>
            {errorMessage ? (
              <Text size="T200" style={{ color: color.Critical.OnContainer }}>
                {errorMessage}
              </Text>
            ) : null}
          </div>
          {onRetry ? (
            <Button variant="Critical" fill="Solid" size="300" onClick={onRetry}>
              <Text size="B300">Retry</Text>
            </Button>
          ) : null}
        </div>
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
        <div
          className={htmlCss.HistoryStatusCard}
          style={{ padding: `${config.space.S100} ${config.space.S300}` }}
          data-timeline-history-status="date"
        >
          <Text size="T200">{visibleDateLabel}</Text>
        </div>
      ) : null}
    </div>
  );
}
