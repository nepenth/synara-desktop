import React, { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Box, Button, Icon, Icons, Input, Spinner, Text, config, toRem } from 'folds';
import { InlineMessage } from '../inline-message';
import {
  settleAgentQuestion,
  toggleAgentQuestionChoice,
  type AgentQuestionAnswer,
  type AgentQuestionSettlement,
  type NativeTimelineAgentQuestion,
} from '../../utils/agentQuestions';
import { answerAgentQuestionWithNativeOwner } from '../../features/room/nativeReactionOwner';
import * as css from './AgentQuestionCard.css';

export type AgentQuestionTarget = {
  roomId: string;
  eventId: string;
};

type AgentQuestionCardProps = {
  question: NativeTimelineAgentQuestion;
  /** Absent when the card cannot be answered from here (no remote event id). */
  target?: AgentQuestionTarget;
  /** Body of this account's latest reply to the card, when one is loaded. */
  ownReplyBody?: string;
  /** Test seam for the native owner route. */
  answer?: typeof answerAgentQuestionWithNativeOwner;
};

const busyKeyFor = (answer: AgentQuestionAnswer): string =>
  answer.kind === 'choice' ? `choice-${answer.index}` : answer.kind;

function SettledLine({ settlement }: { settlement: AgentQuestionSettlement }) {
  const { t } = useTranslation();
  let text: string;
  switch (settlement.kind) {
    case 'answered':
      text = settlement.text
        ? t('modernization.question.answered', 'Answered: {{answer}}', {
            answer: settlement.text,
          })
        : t('modernization.question.answered_plain', 'Answered');
      break;
    case 'skipped':
      text = t('modernization.question.skipped', 'Skipped');
      break;
    case 'expired':
      text = t('modernization.question.expired', 'No answer (expired)');
      break;
    case 'cancelled':
      text = t('modernization.question.cancelled', 'Cancelled');
      break;
    case 'pending':
      return null;
  }
  return (
    <Text size="T300" priority="400" role="status" data-question-settled={settlement.kind}>
      {text}
    </Text>
  );
}

export function AgentQuestionCard({
  question,
  target,
  ownReplyBody,
  answer: answerQuestion = answerAgentQuestionWithNativeOwner,
}: AgentQuestionCardProps) {
  const { t } = useTranslation();
  const [busyKey, setBusyKey] = useState<string>();
  const [localAnswer, setLocalAnswer] = useState<AgentQuestionAnswer>();
  const [selection, setSelection] = useState<number[]>([]);
  const [otherText, setOtherText] = useState('');
  const [error, setError] = useState<string>();
  const settlement = settleAgentQuestion(question, {
    now: Date.now(),
    ownReplyBody,
    localAnswer,
  });
  const pending = settlement.kind === 'pending';
  const interactive = pending && question.canAnswer && Boolean(target);
  const disabled = !interactive || Boolean(busyKey);
  const choices = question.choices ?? [];

  const submit = useCallback(
    async (answer: AgentQuestionAnswer) => {
      if (!target || busyKey || localAnswer) return;
      setBusyKey(busyKeyFor(answer));
      setError(undefined);
      try {
        await answerQuestion({ roomId: target.roomId, eventId: target.eventId, answer });
        // No success alert: the card itself turns into its settled line.
        setLocalAnswer(answer);
      } catch {
        setError(t('modernization.question.failed', 'Your answer could not be sent. Try again.'));
      } finally {
        setBusyKey(undefined);
      }
    },
    [answerQuestion, busyKey, localAnswer, t, target]
  );

  const spinner = (key: string, variant: 'Primary' | 'Secondary') =>
    busyKey === key ? <Spinner size="100" variant={variant} fill="Soft" /> : undefined;

  const otherAnswer = otherText.trim();
  const sendOther = () => {
    if (otherAnswer) void submit({ kind: 'other', text: otherAnswer });
  };

  return (
    <Box
      direction="Column"
      gap="300"
      className={css.QuestionCard}
      style={{ maxWidth: toRem(760), padding: config.space.S400 }}
      role="group"
      aria-label={t('modernization.question.aria_label', 'Question from the agent')}
      data-agent-question-source={question.source}
    >
      <Text size="T400" className={css.QuestionText}>
        {question.question}
      </Text>

      {pending && !interactive && choices.length > 0 && (
        <ol className={css.ChoiceList}>
          {choices.map((choice) => (
            <li key={choice.index} value={choice.index}>
              <Text as="span" size="T300">
                {choice.label}
                {choice.recommended
                  ? ` (${t('modernization.question.recommended', 'Recommended')})`
                  : ''}
              </Text>
            </li>
          ))}
        </ol>
      )}

      {interactive && choices.length > 0 && (
        <Box gap="200" wrap="Wrap" alignItems="Center">
          {choices.map((choice) => {
            const selected = selection.includes(choice.index);
            const label = choice.recommended
              ? t('modernization.question.choice_recommended_aria', '{{label}} (recommended)', {
                  label: choice.label,
                })
              : choice.label;
            if (question.multiSelect) {
              return (
                <Button
                  key={choice.index}
                  type="button"
                  size="300"
                  radii="300"
                  variant={selected ? 'Primary' : 'Secondary'}
                  fill={selected ? 'Soft' : 'None'}
                  outlined
                  disabled={disabled}
                  aria-pressed={selected}
                  aria-label={label}
                  before={selected ? <Icon size="100" src={Icons.Check} /> : undefined}
                  onClick={() => setSelection(toggleAgentQuestionChoice(selection, choice.index))}
                >
                  <Text size="B300">{choice.label}</Text>
                  {choice.recommended && (
                    <Text size="T200" priority="300">
                      {t('modernization.question.recommended', 'Recommended')}
                    </Text>
                  )}
                </Button>
              );
            }
            const key = `choice-${choice.index}`;
            const variant = choice.recommended ? 'Primary' : 'Secondary';
            return (
              <Button
                key={choice.index}
                type="button"
                size="300"
                radii="300"
                variant={variant}
                // Soft, like the other choices: a solid button reads as already picked.
                fill="Soft"
                disabled={disabled}
                aria-label={label}
                before={spinner(key, variant)}
                onClick={() => void submit({ kind: 'choice', index: choice.index })}
              >
                <Text size="B300">{choice.label}</Text>
                {choice.recommended && (
                  <Text size="T200" priority="300">
                    {t('modernization.question.recommended', 'Recommended')}
                  </Text>
                )}
              </Button>
            );
          })}
          {question.multiSelect && (
            <Button
              type="button"
              size="300"
              radii="300"
              variant="Primary"
              fill="Solid"
              disabled={disabled || selection.length === 0}
              aria-label={t('modernization.question.send_selection', 'Send selected answers')}
              before={spinner('choices', 'Primary') ?? <Icon size="100" src={Icons.Send} />}
              onClick={() => void submit({ kind: 'choices', indices: selection })}
            >
              <Text size="B300">{t('modernization.question.send', 'Send')}</Text>
            </Button>
          )}
        </Box>
      )}

      {interactive && question.allowOther && (
        <form
          className={css.OtherRow}
          onSubmit={(event) => {
            event.preventDefault();
            sendOther();
          }}
        >
          <Input
            value={otherText}
            onChange={(event: React.ChangeEvent<HTMLInputElement>) =>
              setOtherText(event.target.value)
            }
            placeholder={t('modernization.question.other_placeholder', 'Other…')}
            aria-label={t('modernization.question.other_aria_label', 'Your own answer')}
            variant="Background"
            size="300"
            radii="300"
            disabled={disabled}
            maxLength={4000}
            style={{ flexGrow: 1 }}
          />
          <Button
            type="submit"
            size="300"
            radii="300"
            variant="Secondary"
            fill="Soft"
            disabled={disabled || !otherAnswer}
            aria-label={t('modernization.question.other_send_aria', 'Send your own answer')}
            before={spinner('other', 'Secondary') ?? <Icon size="100" src={Icons.Send} />}
          >
            <Text size="B300">{t('modernization.question.send', 'Send')}</Text>
          </Button>
        </form>
      )}

      {interactive && question.allowSkip && (
        <Box gap="200" alignItems="Center">
          <span className={css.ActionSpacer} aria-hidden />
          <Button
            type="button"
            size="300"
            radii="300"
            variant="Secondary"
            fill="None"
            disabled={disabled}
            before={spinner('skip', 'Secondary')}
            onClick={() => void submit({ kind: 'skip' })}
          >
            <Text size="B300">{t('modernization.question.skip', 'Skip')}</Text>
          </Button>
        </Box>
      )}

      {pending && !question.canAnswer && (
        <Text size="T200" priority="300">
          {t('modernization.question.read_only', 'This question is for someone else.')}
        </Text>
      )}
      <SettledLine settlement={settlement} />
      {error && <InlineMessage tone="critical">{error}</InlineMessage>}
    </Box>
  );
}
