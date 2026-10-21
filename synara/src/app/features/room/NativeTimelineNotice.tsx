import React from 'react';
import { Box, Button, Icon, IconButton, Icons, Text } from 'folds';
import * as htmlCss from './nativeTimelineHtml.css';

export type NativeTimelineNoticeState = { tone: 'error' | 'info'; message: string };

/** A failed row action or a routing notice, shown above the timeline or in a thread. */
export const NativeTimelineNotice = ({
  tone,
  message,
  action,
  onDismiss,
}: NativeTimelineNoticeState & {
  action?: { label: string; onClick: () => void; disabled?: boolean };
  onDismiss?: () => void;
}) => (
  <Box
    className={htmlCss.TimelineNotice({ tone })}
    gap="200"
    alignItems="Center"
    role={tone === 'error' ? 'alert' : 'status'}
    data-native-timeline-notice={tone}
  >
    <Icon src={tone === 'error' ? Icons.Warning : Icons.Info} size="100" />
    <Box grow="Yes">
      <Text size="T200">{message}</Text>
    </Box>
    {action ? (
      <Button
        size="300"
        variant={tone === 'error' ? 'Critical' : 'Secondary'}
        fill="Soft"
        radii="300"
        disabled={action.disabled}
        onClick={action.onClick}
      >
        <Text size="B300">{action.label}</Text>
      </Button>
    ) : null}
    {onDismiss ? (
      <IconButton
        onClick={onDismiss}
        variant={tone === 'error' ? 'Critical' : 'SurfaceVariant'}
        fill="Soft"
        size="300"
        radii="300"
        aria-label="Dismiss"
      >
        <Icon src={Icons.Cross} size="50" />
      </IconButton>
    ) : null}
  </Box>
);
