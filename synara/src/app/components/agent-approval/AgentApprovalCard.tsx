import React, { useCallback, useState } from 'react';
import { Box, Button, Chip, Spinner, Text, color, config, toRem } from 'folds';
import {
  AGENT_APPROVAL_REACTION_APPROVE_ALWAYS,
  AGENT_APPROVAL_REACTION_APPROVE_ONCE,
  AGENT_APPROVAL_REACTION_DENY,
  type AgentApprovalPrompt,
} from '../../utils/agentApprovals';
import { getReactionContent } from '../../utils/room';
import { useMatrixClient } from '../../hooks/useMatrixClient';
import { MessageEvent } from '../../../types/matrix/room';

export type AgentApprovalTarget = {
  roomId: string;
  eventId: string;
  canSendReaction?: boolean;
};

type AgentApprovalCardProps = {
  prompt: AgentApprovalPrompt;
  target?: AgentApprovalTarget;
};

type ApprovalAction = {
  key: string;
  label: string;
  variant: 'Primary' | 'Critical' | 'Secondary';
};

const APPROVAL_ACTIONS: ApprovalAction[] = [
  {
    key: AGENT_APPROVAL_REACTION_APPROVE_ONCE,
    label: 'Approve once',
    variant: 'Primary',
  },
  {
    key: AGENT_APPROVAL_REACTION_APPROVE_ALWAYS,
    label: 'Approve always',
    variant: 'Primary',
  },
  {
    key: AGENT_APPROVAL_REACTION_DENY,
    label: 'Deny',
    variant: 'Critical',
  },
];

export function AgentApprovalCard({ prompt, target }: AgentApprovalCardProps) {
  const mx = useMatrixClient();
  const [busyKey, setBusyKey] = useState<string>();
  const [sentKey, setSentKey] = useState<string>();
  const [error, setError] = useState<string>();
  const canReact = Boolean(target && target.canSendReaction !== false);
  const disabled = !canReact || Boolean(busyKey || sentKey);

  const handleReact = useCallback(
    async (reactionKey: string) => {
      if (!target || target.canSendReaction === false || busyKey || sentKey) return;

      setBusyKey(reactionKey);
      setError(undefined);
      try {
        await mx.sendEvent(
          target.roomId,
          MessageEvent.Reaction as any,
          getReactionContent(target.eventId, reactionKey) as any
        );
        setSentKey(reactionKey);
      } catch {
        setError('Failed to send approval reaction.');
      } finally {
        setBusyKey(undefined);
      }
    },
    [mx, target, busyKey, sentKey]
  );

  return (
    <Box
      direction="Column"
      gap="300"
      style={{
        maxWidth: toRem(760),
        border: `${config.borderWidth.B300} solid ${color.Critical.Main}`,
        borderRadius: config.radii.R400,
        padding: config.space.S400,
        backgroundColor: color.Surface.Container,
      }}
    >
      <Box justifyContent="SpaceBetween" alignItems="Start" gap="300">
        <Box direction="Column" gap="100" grow="Yes" style={{ minWidth: 0 }}>
          <Text size="H5">{prompt.title}</Text>
          <Text priority="400">{prompt.body}</Text>
        </Box>
        <Chip variant="Critical" radii="Pill" outlined>
          <Text size="L400">Critical</Text>
        </Chip>
      </Box>

      {prompt.command && (
        <Box
          as="details"
          direction="Column"
          gap="200"
          style={{
            border: `${config.borderWidth.B300} solid ${color.SurfaceVariant.ContainerLine}`,
            borderRadius: config.radii.R300,
            padding: config.space.S300,
          }}
        >
          <Box as="summary" alignItems="Center" gap="200" style={{ cursor: 'pointer' }}>
            <Text as="span" size="T300" truncate>
              {prompt.commandPreview ?? 'Review command'}
            </Text>
          </Box>
          <pre
            style={{
              maxWidth: toRem(720),
              overflow: 'auto',
              whiteSpace: 'pre-wrap',
              wordBreak: 'break-word',
              margin: 0,
              padding: config.space.S300,
              borderRadius: config.radii.R300,
              backgroundColor: color.SurfaceVariant.Container,
            }}
          >
            <code>{prompt.command}</code>
          </pre>
        </Box>
      )}

      <Box direction="Column" gap="200">
        <Box gap="200" wrap="Wrap">
          {APPROVAL_ACTIONS.map((action) => (
            <Button
              key={action.key}
              type="button"
              size="300"
              variant={action.variant}
              fill={action.variant === 'Critical' ? 'Solid' : 'Soft'}
              disabled={disabled}
              before={
                busyKey === action.key ? <Spinner size="100" variant={action.variant} /> : undefined
              }
              onClick={() => handleReact(action.key)}
            >
              <Text size="B300">{`${action.key} ${action.label}`}</Text>
            </Button>
          ))}
        </Box>
        {!target && (
          <Text size="T200" priority="300">
            Open the room message to approve by reaction.
          </Text>
        )}
        {target?.canSendReaction === false && (
          <Text size="T200" style={{ color: color.Critical.Main }}>
            You do not have permission to react in this room.
          </Text>
        )}
        {sentKey && (
          <Text size="T200" priority="300">
            Sent {sentKey} reaction.
          </Text>
        )}
        {error && (
          <Text size="T200" style={{ color: color.Critical.Main }}>
            {error}
          </Text>
        )}
      </Box>
    </Box>
  );
}
