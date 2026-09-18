import React, { useEffect, useState } from 'react';
import { Button, Spinner, Text, color, config } from 'folds';
import type { TimelineHistoryOverlayKind } from '../../utils/timelinePagination';
import * as htmlCss from './nativeTimelineHtml.css';

const LOADING_CHROME_DELAY_MS = 320;
const LOADING_CHROME_HOLD_MS = 480;

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

const edgeInsetStyle = (reserveRail?: boolean): React.CSSProperties | undefined =>
  reserveRail ? undefined : { right: 0 };

function NativeTimelineHistoryStatusView({
  edge,
  kind,
  errorMessage,
  visibleDateLabel,
  reserveRail,
  onRetry,
  onLoadMore,
}: NativeTimelineHistoryStatusProps) {
  const [loadingVisible, setLoadingVisible] = useState(false);
  useEffect(() => {
    if (kind === 'loading') {
      // Historical (top) loading should be visible immediately. Newer-message
      // chrome at the live tail is delayed so a single wheel tick cannot flash.
      if (edge === 'backward') {
        setLoadingVisible(true);
        return undefined;
      }
      const timer = window.setTimeout(() => setLoadingVisible(true), LOADING_CHROME_DELAY_MS);
      return () => window.clearTimeout(timer);
    }
    // Live-tail chrome must drop immediately. Holding it after a rejected
    // forward page is what made "Loading newer messages" strobe on overscroll.
    if (edge === 'forward') {
      setLoadingVisible(false);
      return undefined;
    }
    const timer = window.setTimeout(() => setLoadingVisible(false), LOADING_CHROME_HOLD_MS);
    return () => window.clearTimeout(timer);
  }, [kind, edge]);
  const paintedKind: TimelineHistoryOverlayKind =
    kind === 'error'
      ? 'error'
      : kind === 'loading' && edge === 'backward'
      ? 'loading'
      : loadingVisible
      ? 'loading'
      : kind === 'loading'
      ? 'hidden'
      : kind;
  const copy = copyForEdge(edge);
  const showDate = Boolean(visibleDateLabel) && kind === 'hidden' && !reserveRail;
  if (paintedKind === 'hidden' && !showDate) return null;

  if (paintedKind === 'hidden' && showDate) {
    return (
      <div className={htmlCss.HistoryStatusDateChip}>
        <div
          className={htmlCss.HistoryStatusDateChipCard}
          style={{ padding: `${config.space.S100} ${config.space.S300}` }}
          data-timeline-history-status="date"
        >
          <Text size="T200">{visibleDateLabel}</Text>
        </div>
      </div>
    );
  }

  return (
    <div
      className={
        edge === 'backward'
          ? htmlCss.HistoryStatusOverlayBackward
          : htmlCss.HistoryStatusOverlayForward
      }
      style={edgeInsetStyle(reserveRail)}
    >
      {paintedKind === 'loading' ? (
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
      {paintedKind === 'error' ? (
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
      {paintedKind === 'load_more' && onLoadMore ? (
        <div className={htmlCss.HistoryStatusHitTarget}>
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
        </div>
      ) : null}
    </div>
  );
}

export const NativeTimelineHistoryStatus = React.memo(NativeTimelineHistoryStatusView);
