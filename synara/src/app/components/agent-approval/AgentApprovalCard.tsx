import React, { useCallback, useEffect, useRef, useState } from 'react';
import { Box, Button, Icon, IconSrc, Icons, Spinner, Text, color, config, toRem } from 'folds';
import { InlineMessage } from '../inline-message';
import {
  AGENT_APPROVAL_REACTION_APPROVE_ALWAYS,
  AGENT_APPROVAL_REACTION_APPROVE_ONCE,
  AGENT_APPROVAL_REACTION_DENY,
  AGENT_APPROVAL_NOTIFICATION_ACTION_APPROVE_ALWAYS,
  AGENT_APPROVAL_NOTIFICATION_ACTION_APPROVE_ONCE,
  AGENT_APPROVAL_NOTIFICATION_ACTION_DENY,
  describeAgentApprovalDecision,
  type AgentApprovalOwnDecision,
  type AgentApprovalPrompt,
} from '../../utils/agentApprovals';
import { decideAgentApprovalWithNativeOwner } from '../../features/room/nativeReactionOwner';
import * as css from './AgentApprovalCard.css';

export type AgentApprovalTarget = {
  roomId: string;
  eventId: string;
  canSendReaction?: boolean;
  /** True only when the Core timeline owner classified this exact event. */
  coreEligible?: boolean;
  /** This account's own decision, projected by Core from its terminal reaction. */
  decision?: AgentApprovalOwnDecision;
};

type AgentApprovalCardProps = {
  prompt: AgentApprovalPrompt;
  target?: AgentApprovalTarget;
  onDecision?: () => void;
  appearance?: 'timeline' | 'inbox';
};

type ApprovalAction = {
  actionId: string;
  key: string;
  label: string;
  icon: IconSrc;
  variant: 'Primary' | 'Critical' | 'Secondary';
};

// Rendered left to right: Deny sits apart on the left, the two approvals on
// the right with Approve once as the primary.
const APPROVAL_ACTIONS: ApprovalAction[] = [
  {
    actionId: AGENT_APPROVAL_NOTIFICATION_ACTION_DENY,
    key: AGENT_APPROVAL_REACTION_DENY,
    label: 'Deny',
    icon: Icons.Cross,
    variant: 'Critical',
  },
  {
    actionId: AGENT_APPROVAL_NOTIFICATION_ACTION_APPROVE_ALWAYS,
    key: AGENT_APPROVAL_REACTION_APPROVE_ALWAYS,
    label: 'Approve always',
    icon: Icons.CheckTwice,
    variant: 'Secondary',
  },
  {
    actionId: AGENT_APPROVAL_NOTIFICATION_ACTION_APPROVE_ONCE,
    key: AGENT_APPROVAL_REACTION_APPROVE_ONCE,
    label: 'Approve once',
    icon: Icons.Check,
    variant: 'Primary',
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
  fontFamily: 'var(--font-mono)',
  fontSize: toRem(12),
  lineHeight: 1.45,
};

export function AgentApprovalCard({
  prompt,
  target,
  onDecision,
  appearance = 'timeline',
}: AgentApprovalCardProps) {
  const [busyKey, setBusyKey] = useState<string>();
  const [sentKey, setSentKey] = useState<string>();
  const [decisionStatus, setDecisionStatus] = useState<'applied' | 'already_decided'>();
  const [error, setError] = useState<string>();
  const [confirmApproveAlways, setConfirmApproveAlways] = useState(false);
  const confirmRef = useRef<HTMLButtonElement>(null);
  const approveAlwaysRef = useRef<HTMLButtonElement>(null);
  const wasConfirming = useRef(false);
  useEffect(() => {
    if (confirmApproveAlways) confirmRef.current?.focus();
    else if (wasConfirming.current) approveAlwaysRef.current?.focus();
    wasConfirming.current = confirmApproveAlways;
  }, [confirmApproveAlways]);
  const canReact = Boolean(target?.coreEligible === true && target.canSendReaction !== false);
  const coreDecision = target?.decision;
  const disabled = !canReact || Boolean(busyKey || sentKey || coreDecision);
  const isResolved = Boolean(sentKey || coreDecision);

  const submitDecision = useCallback(
    async (action: ApprovalAction) => {
      if (
        !target?.coreEligible ||
        target.canSendReaction === false ||
        busyKey ||
        sentKey ||
        target.decision
      )
        return;

      setBusyKey(action.key);
      setError(undefined);
      try {
        const result = await decideAgentApprovalWithNativeOwner({
          roomId: target.roomId,
          eventId: target.eventId,
          actionId: action.actionId,
        });
        setDecisionStatus(result.status);
        setSentKey(action.key);
        setConfirmApproveAlways(false);
        onDecision?.();
      } catch {
        setError('Approval could not be submitted. The request may be invalid or expired.');
      } finally {
        setBusyKey(undefined);
      }
    },
    [target, busyKey, sentKey, onDecision]
  );

  const handleReact = useCallback(
    async (action: ApprovalAction) => {
      if (!target?.coreEligible || target.canSendReaction === false || busyKey || sentKey) return;

      // Permanent approval requires an explicit second confirmation step.
      if (action.key === AGENT_APPROVAL_REACTION_APPROVE_ALWAYS && !confirmApproveAlways) {
        setConfirmApproveAlways(true);
        setError(undefined);
        return;
      }

      if (action.key !== AGENT_APPROVAL_REACTION_APPROVE_ALWAYS && confirmApproveAlways) {
        setConfirmApproveAlways(false);
      }

      await submitDecision(action);
    },
    [target, busyKey, sentKey, confirmApproveAlways, submitDecision]
  );

  return (
    <Box
      direction="Column"
      gap="300"
      className={appearance === 'inbox' ? css.InboxApprovalCard : css.ApprovalCard}
      style={{
        maxWidth: appearance === 'inbox' ? undefined : toRem(760),
        padding: config.space.S400,
      }}
    >
      <Box direction="Column" gap="100" style={{ minWidth: 0 }}>
        {appearance === 'inbox' ? (
          <Text size="T300" className={css.PromptTitle}>
            {prompt.title}
          </Text>
        ) : (
          <Text size="H5">{prompt.title}</Text>
        )}
        <Text priority="400">{prompt.body}</Text>
      </Box>

      {prompt.command && !isResolved && (
        <Box direction="Column" gap="100">
          <Text size="L400" priority="300">
            Command
          </Text>
          <pre style={monospacedBlockStyle}>
            <code>{prompt.command}</code>
          </pre>
        </Box>
      )}

      {!prompt.command && prompt.sourceContext && !isResolved && (
        <Box direction="Column" gap="200">
          <Text size="L400">Full approval request</Text>
          <pre style={monospacedBlockStyle}>
            <code>{prompt.sourceContext}</code>
          </pre>
        </Box>
      )}

      <Box direction="Column" gap="200">
        {confirmApproveAlways && (
          <Box direction="Column" gap="200" className={css.ConfirmAlways}>
            <Text size="T300" priority="400">
              This trusts the command pattern permanently. The agent runs matching commands without
              asking again.
            </Text>
            <Box gap="200" wrap="Wrap" justifyContent="End">
              <Button
                type="button"
                size="300"
                radii="300"
                variant="Secondary"
                fill="Soft"
                disabled={Boolean(busyKey || sentKey)}
                onClick={() => setConfirmApproveAlways(false)}
              >
                <Text size="B300">Cancel</Text>
              </Button>
              <Button
                ref={confirmRef}
                type="button"
                size="300"
                radii="300"
                variant="Primary"
                fill="Solid"
                disabled={disabled}
                before={
                  busyKey === AGENT_APPROVAL_REACTION_APPROVE_ALWAYS ? (
                    <Spinner size="100" variant="Primary" fill="Solid" />
                  ) : undefined
                }
                onClick={() =>
                  handleReact(
                    APPROVAL_ACTIONS.find(
                      (action) => action.key === AGENT_APPROVAL_REACTION_APPROVE_ALWAYS
                    )!
                  )
                }
              >
                <Text size="B300">Confirm approve always</Text>
              </Button>
            </Box>
          </Box>
        )}

        {!isResolved && (
          <Box gap="200" wrap="Wrap" alignItems="Center">
            {APPROVAL_ACTIONS.map((action) => {
              const isAlways = action.key === AGENT_APPROVAL_REACTION_APPROVE_ALWAYS;
              const isDeny = action.key === AGENT_APPROVAL_REACTION_DENY;
              const isOnce = action.key === AGENT_APPROVAL_REACTION_APPROVE_ONCE;
              const hideWhileConfirmingAlways = confirmApproveAlways && isAlways;
              if (hideWhileConfirmingAlways) return null;

              return (
                <React.Fragment key={action.key}>
                  <Button
                    ref={isAlways ? approveAlwaysRef : undefined}
                    type="button"
                    size="300"
                    radii="300"
                    variant={action.variant}
                    fill={isOnce ? 'Solid' : isDeny ? 'None' : 'Soft'}
                    outlined={isDeny}
                    disabled={disabled}
                    before={
                      busyKey === action.key ? (
                        <Spinner
                          size="100"
                          variant={action.variant}
                          fill={isOnce ? 'Solid' : 'Soft'}
                        />
                      ) : (
                        <Icon size="100" src={action.icon} />
                      )
                    }
                    onClick={() => handleReact(action)}
                  >
                    <Text size="B300">{action.label}</Text>
                  </Button>
                  {isDeny && <span className={css.ActionSpacer} aria-hidden />}
                </React.Fragment>
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
          <InlineMessage tone="critical" role="status">
            You do not have permission to react in this room.
          </InlineMessage>
        )}
        {!isResolved && (
          <details className={css.MoreWays}>
            <summary className={css.MoreWaysSummary}>
              <Text as="span" size="T200" priority="300">
                More ways to approve
              </Text>
            </summary>
            <Text size="T200" priority="300">
              To approve for this session only, open the message and reply{' '}
              <code>!approve session</code>.
            </Text>
          </details>
        )}
        {isResolved && (
          <Box direction="Column" gap="100">
            <Text size="T300" priority="400">
              {coreDecision
                ? describeAgentApprovalDecision(coreDecision)
                : decisionStatus === 'already_decided'
                  ? 'This approval was already decided on this account.'
                  : `Decision sent: ${
                      APPROVAL_ACTIONS.find((action) => action.key === sentKey)?.label ?? sentKey
                    }.`}
            </Text>
            {prompt.commandPreview && (
              <Text size="T200" priority="300">
                Command: {prompt.commandPreview}
              </Text>
            )}
          </Box>
        )}
        {error && <InlineMessage tone="critical">{error}</InlineMessage>}
      </Box>
    </Box>
  );
}
