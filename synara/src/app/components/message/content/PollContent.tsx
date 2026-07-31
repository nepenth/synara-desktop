import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { Badge, Box, Button, Icon, Icons, ProgressBar, Text, config } from 'folds';
import type { MatrixEvent } from 'matrix-js-sdk/lib/models/event';
import { useTranslation } from 'react-i18next';
import { useMatrixClient } from '../../../hooks/useMatrixClient';
import { respondPollWithNativeDesktopOwner } from '../../../features/room/nativePoll';
import { ParsedPoll, parsePollResponseContent, summarizePollResponses } from '../../../utils/polls';

export type PollContentProps = {
  roomId: string;
  eventId: string;
  poll: ParsedPoll;
};

export function PollContent({ roomId, eventId, poll }: PollContentProps) {
  const mx = useMatrixClient();
  const { t } = useTranslation();
  const [isSending, setIsSending] = useState(false);
  const [responses, setResponses] = useState<
    Array<{ sender?: string; ts?: number; answers: string[] }>
  >([]);
  const [error, setError] = useState<string>();

  const answerIds = useMemo(() => poll.answers.map((answer) => answer.id), [poll.answers]);
  const counts = useMemo(
    () => summarizePollResponses(responses, answerIds),
    [responses, answerIds]
  );
  const totalVotes = Object.values(counts).reduce((total, count) => total + count, 0);
  const myUserId = mx.getUserId() ?? undefined;
  const myAnswerIds = useMemo(() => {
    let latest: { ts?: number; answers: string[] } | undefined;
    responses.forEach((response) => {
      if (response.sender !== myUserId) return;
      if (!latest || (response.ts ?? 0) >= (latest.ts ?? 0)) {
        latest = response;
      }
    });

    const latestAnswers = latest?.answers ?? [];
    const uniqueAnswers = new Set<string>();
    latestAnswers.forEach((answerId) => {
      if (answerIds.includes(answerId)) uniqueAnswers.add(answerId);
    });
    return uniqueAnswers;
  }, [answerIds, myUserId, responses]);

  const loadResponses = useCallback(async () => {
    try {
      const relationResult = await mx.relations(roomId, eventId, 'm.reference', undefined);
      const nextResponses: Array<{ sender?: string; ts?: number; answers: string[] } | undefined> =
        relationResult.events.map((event: MatrixEvent) => {
          const answers = parsePollResponseContent(event.getContent<Record<string, unknown>>());
          if (!answers) return undefined;
          return {
            sender: event.getSender() ?? undefined,
            ts: event.getTs(),
            answers,
          };
        });
      setResponses(
        nextResponses.filter(
          (response): response is { sender?: string; ts?: number; answers: string[] } =>
            Boolean(response)
        )
      );
    } catch {
      setError(t('modernization.poll.load_failed', 'Could not load poll responses.'));
    }
  }, [mx, roomId, eventId, t]);

  useEffect(() => {
    loadResponses();
  }, [loadResponses]);

  const handleVote = async (answerId: string) => {
    const currentlySelected = myAnswerIds.has(answerId);
    const nextAnswerIds = new Set(myAnswerIds);

    if (currentlySelected) {
      nextAnswerIds.delete(answerId);
    } else {
      if (nextAnswerIds.size >= poll.maxSelections) {
        setError(
          t('modernization.poll.selection_limit', {
            count: poll.maxSelections,
          })
        );
        return;
      }
      nextAnswerIds.add(answerId);
    }

    const orderedAnswers = answerIds.filter((id) => nextAnswerIds.has(id));
    setIsSending(true);
    setError(undefined);
    try {
      const owner = await respondPollWithNativeDesktopOwner({
        roomId,
        pollEventId: eventId,
        answerIds: orderedAnswers,
      });
      if (owner === 'legacy') {
        setError(
          t(
            'modernization.poll.native_required',
            'Native Matrix session is required to vote on desktop.'
          )
        );
        return;
      }
      setResponses((current) => [
        ...current,
        {
          sender: myUserId,
          ts: Date.now(),
          answers: orderedAnswers,
        },
      ]);
    } catch {
      setError(t('modernization.poll.vote_failed', 'Could not send your vote.'));
    } finally {
      setIsSending(false);
    }
  };

  return (
    <Box
      direction="Column"
      gap="300"
      style={{
        maxWidth: '34rem',
        padding: config.space.S300,
        borderRadius: config.radii.R400,
        border: `${config.borderWidth.B300} solid currentColor`,
      }}
      role="group"
      aria-label={t('modernization.poll.aria_label', 'Poll')}
    >
      <Box alignItems="Center" gap="200">
        <Icon size="100" src={Icons.Message} />
        <Text size="H5">{poll.question}</Text>
      </Box>
      <Text size="T200" priority="300">
        {poll.maxSelections === 1
          ? t('modernization.poll.selection_single', 'Select one option')
          : t('modernization.poll.selection_multiple', {
              count: poll.maxSelections,
              defaultValue: 'Select up to {{count}} options',
            })}
      </Text>
      <Box direction="Column" gap="200">
        {poll.answers.map((answer) => {
          const count = counts[answer.id] ?? 0;
          const percent = totalVotes > 0 ? Math.round((count / totalVotes) * 100) : 0;
          const selected = myAnswerIds.has(answer.id);

          return (
            <Button
              key={answer.id}
              size="300"
              variant={selected ? 'Primary' : 'Secondary'}
              fill="None"
              radii="300"
              disabled={isSending}
              aria-pressed={selected}
              aria-label={
                selected
                  ? t('modernization.poll.unvote_aria_label', {
                      option: answer.text,
                      defaultValue: `Deselect ${answer.text}`,
                    })
                  : t('modernization.poll.vote_aria_label', {
                      option: answer.text,
                      defaultValue: `Select ${answer.text}`,
                    })
              }
              onClick={() => handleVote(answer.id)}
            >
              <Box grow="Yes" direction="Column" gap="100">
                <Box alignItems="Center" gap="200">
                  <Text size="B300" truncate>
                    {answer.text}
                  </Text>
                  <Badge size="400" variant="Secondary" radii="Pill">
                    <Text size="L400">{count}</Text>
                  </Badge>
                </Box>
                <ProgressBar
                  as="div"
                  variant={selected ? 'Primary' : 'Secondary'}
                  size="300"
                  min={0}
                  max={100}
                  value={percent}
                  radii="300"
                />
              </Box>
            </Button>
          );
        })}
      </Box>
      <Text size="T200" priority="300">
        {t('modernization.poll.vote_count', { count: totalVotes, defaultValue: '{{count}} votes' })}
      </Text>
      {error && (
        <Text size="T300" style={{ color: 'currentColor' }}>
          {error}
        </Text>
      )}
    </Box>
  );
}
