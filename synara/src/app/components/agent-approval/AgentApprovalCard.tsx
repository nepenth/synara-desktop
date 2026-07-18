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

const monospacedBlockStyle: React.CSSProperties = {
  maxWidth: toRem(720),
  maxHeight: toRem(220),
  overflow: 'auto',
  whiteSpace: 'pre-wrap',
  wordBreak: 'break-word',
  margin: 0,
  padding: config.space.S300,
  borderRadius: config.radii.R300,
  backgroundColor: color.SurfaceVariant.Container,
  fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace',
  fontSize: toRem(12),
  lineHeight: 1.45,
};

export function AgentApprovalCard({ prompt, target }: AgentApprovalCardProps) {
  const mx = useMatrixClient();
  const [busyKey, setBusyKey] = useState<string>();
  const [sentKey, setSentKey] = useState<string>();
  const [error, setError] = useState<string>();
  const [confirmApproveAlways, setConfirmApproveAlways] = useState(false);
  const canReact = Boolean(target && target.canSendReaction !== false);
  const disabled = !canReact || Boolean(busyKey || sentKey);
  const isResolved = Boolean(sentKey);

  // Default closed for huge source contexts (common on first real large prompts).
  const [showFullPrompt, setShowFullPrompt] = useState(false);

  const sendReaction = useCallback(
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
        setConfirmApproveAlways(false);
      } catch {
        setError('Failed to send approval reaction.');
      } finally {
        setBusyKey(undefined);
      }
    },
    [mx, target, busyKey, sentKey]
  );

  const handleReact = useCallback(
    async (reactionKey: string) => {
      if (!target || target.canSendReaction === false || busyKey || sentKey) return;

      // Permanent approval requires an explicit second confirmation step.
      if (reactionKey === AGENT_APPROVAL_REACTION_APPROVE_ALWAYS && !confirmApproveAlways) {
        setConfirmApproveAlways(true);
        setError(undefined);
        return;
      }

      if (reactionKey !== AGENT_APPROVAL_REACTION_APPROVE_ALWAYS && confirmApproveAlways) {
        setConfirmApproveAlways(false);
      }

      await sendReaction(reactionKey);
    },
    [target, busyKey, sentKey, confirmApproveAlways, sendReaction]
  );

  const sourceDetails =
    prompt.replyInstructions ||
    (prompt.sourceContext && prompt.sourceContext !== prompt.body
      ? prompt.sourceContext
      : undefined);

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

      {prompt.command && !isResolved && (
        <Box
          direction="Column"
          gap="200"
          style={{
            border: `${config.borderWidth.B300} solid ${color.SurfaceVariant.ContainerLine}`,
            borderRadius: config.radii.R300,
            padding: config.space.S300,
          }}
        >
          <Text size="L400" priority="300">
            Command{prompt.commandPreview ? `: ${prompt.commandPreview}` : ''}
          </Text>
          <pre style={monospacedBlockStyle}>
            <code>{prompt.command}</code>
          </pre>
        </Box>
      )}

      {sourceDetails && !isResolved && (
        <Box
          as="details"
          direction="Column"
          gap="200"
          style={{
            border: `${config.borderWidth.B300} solid ${color.SurfaceVariant.ContainerLine}`,
            borderRadius: config.radii.R300,
            padding: config.space.S300,
          }}
          open={false}
        >
          <Box as="summary" alignItems="Center" gap="200" style={{ cursor: 'pointer' }}>
            <Text as="span" size="T300">
              Full approval prompt
            </Text>
          </Box>
          {prompt.replyInstructions && (
            <Box direction="Column" gap="100">
              <Text size="L400" priority="300">
                Reply / reaction options
              </Text>
              <pre style={{ ...monospacedBlockStyle, maxHeight: toRem(140) }}>
                <code>{prompt.replyInstructions}</code>
              </pre>
            </Box>
          )}
          {prompt.sourceContext && (
            <Box direction="Column" gap="100">
              <Text size="L400" priority="300">
                Source context
              </Text>
              <pre style={monospacedBlockStyle}>
                <code>{prompt.sourceContext}</code>
              </pre>
            </Box>
          )}
        </Box>
      )}

      <Box direction="Column" gap="200">
        {confirmApproveAlways && (
          <Box
            direction="Column"
            gap="200"
            style={{
              border: `${config.borderWidth.B300} solid ${color.Critical.Main}`,
              borderRadius: config.radii.R300,
              padding: config.space.S300,
              backgroundColor: color.Critical.Container,
            }}
          >
            <Text size="T300" priority="400">
              Approve always permanently trusts this command pattern. Confirm only if you intend to
              allow it without future prompts.
            </Text>
            <Box gap="200" wrap="Wrap">
              <Button
                type="button"
                size="300"
                variant="Critical"
                fill="Solid"
                disabled={disabled}
                before={
                  busyKey === AGENT_APPROVAL_REACTION_APPROVE_ALWAYS ? (
                    <Spinner size="100" variant="Critical" />
                  ) : undefined
                }
                onClick={() => handleReact(AGENT_APPROVAL_REACTION_APPROVE_ALWAYS)}
              >
                <Text size="B300">
                  {AGENT_APPROVAL_REACTION_APPROVE_ALWAYS} Confirm approve always
                </Text>
              </Button>
              <Button
                type="button"
                size="300"
                variant="Secondary"
                fill="Soft"
                disabled={Boolean(busyKey || sentKey)}
                onClick={() => setConfirmApproveAlways(false)}
              >
                <Text size="B300">Cancel</Text>
              </Button>
            </Box>
          </Box>
        )}

        {!isResolved && (
        <Box gap="200" wrap="Wrap">
          {APPROVAL_ACTIONS.map((action) => {
            const isAlways = action.key === AGENT_APPROVAL_REACTION_APPROVE_ALWAYS;
            const hideWhileConfirmingAlways = confirmApproveAlways && isAlways;
            if (hideWhileConfirmingAlways) return null;

            return (
              <Button
                key={action.key}
                type="button"
                size="300"
                variant={action.variant}
                fill={action.variant === 'Critical' ? 'Solid' : 'Soft'}
                disabled={disabled}
                before={
                  busyKey === action.key ? (
                    <Spinner size="100" variant={action.variant} />
                  ) : undefined
                }
                onClick={() => handleReact(action.key)}
              >
                <Text size="B300">{`${action.key} ${action.label}`}</Text>
              </Button>
            );
          })}
        </Box>
        )}

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
        {isResolved && (
          <Box
            direction="Column"
            gap="100"
            style={{
              border: `${config.borderWidth.B300} solid ${color.Success?.Main || "#22c55e"}`,
              borderRadius: config.radii.R300,
              padding: config.space.S200,
              backgroundColor: color.SurfaceVariant?.Container || "rgba(0,0,0,0.03)",
            }}
          >
            <Text size="T300" priority="400">
              Approved ({sentKey}). Card can be dismissed or will be replaced in a future update.
            </Text>
            {prompt.commandPreview && (
              <Text size="T200" priority="300">Command: {prompt.commandPreview}</Text>
            )}
          </Box>
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
