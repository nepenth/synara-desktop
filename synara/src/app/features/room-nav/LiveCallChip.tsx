import React from 'react';
import { Badge, Text } from 'folds';
import { liveCallChipLabel } from '../matrix-rtc/liveCallChrome';

type LiveCallChipProps = {
  participantCount: number;
};

export function LiveCallChip({ participantCount }: LiveCallChipProps) {
  return (
    <Badge
      size="300"
      variant="Success"
      fill="Soft"
      radii="Pill"
      outlined
      data-testid="live-call-chip"
    >
      <Text size="T200">{liveCallChipLabel(participantCount)}</Text>
    </Badge>
  );
}

export { liveCallChipLabel };
