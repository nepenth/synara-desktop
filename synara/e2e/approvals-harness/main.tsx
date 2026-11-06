// Production presenters and composer layout/styles with fixture data and preview actions.
import React, { useState } from 'react';
import { createRoot } from 'react-dom/client';
import { MemoryRouter, useNavigate } from 'react-router-dom';
import { decideAgentApprovalWithNativeOwner } from '../../src/app/features/room/nativeReactionOwner';
import { Box, Icon, IconButton, Icons, Text, configClass, varsClass, color } from 'folds';
import 'folds/dist/style.css';
import '../../src/index.css';
import { darkTheme, synaraLightTheme } from '../../src/colors.css';
import {
  ApprovalInboxProvider,
  useApprovalInbox,
} from '../../src/app/features/approvals/ApprovalInboxProvider';
import { ApprovalsView } from '../../src/app/features/approvals/Approvals';
import type { ApprovalInboxItem } from '../../src/app/features/approvals/nativeApprovalInbox';
import { ApprovalsTab } from '../../src/app/pages/client/sidebar/ApprovalsTab';
import {
  Sidebar,
  SidebarStack,
  SidebarAvatar,
  SidebarItem,
  SidebarStackSeparator,
} from '../../src/app/components/sidebar';
import { useEditor } from '../../src/app/components/editor/Editor';
import { Toolbar } from '../../src/app/components/editor/Toolbar';
import { RoomComposer } from '../../src/app/features/room/RoomComposer';
import * as composerCss from '../../src/app/features/room/RoomComposer.css';
import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import { AgentQuestionCard } from '../../src/app/components/agent-question/AgentQuestionCard';
import type { NativeTimelineAgentQuestion } from '../../src/app/utils/agentQuestions';

const query = new URLSearchParams(location.search);
if (query.has('composer')) document.title = 'Synara · Composer review';
// Question cards read their copy through react-i18next; load the shipped
// English catalog before the first render so the fixture shows real strings.
if (query.has('questions')) {
  document.title = 'Synara · Question cards';
  const enLocale = await fetch('/locales/en.json').then((response) => response.json());
  await i18n.use(initReactI18next).init({
    lng: 'en',
    resources: { en: { translation: enLocale } },
    interpolation: { escapeValue: false },
  });
}
const questionAnswers: Record<string, unknown>[] = [];
Object.assign(window, { synaraQuestionAnswers: questionAnswers });
let items: ApprovalInboxItem[] = [];
let failed = query.has('error');
let loading = query.has('loading');
let incomplete = query.has('partial');
let sequence = 0;
let decisions = 0;
let lastAction = '';
let sessionGeneration = 1;
let finishDecision: (() => void) | undefined;
let lastDiscoveryActive = false;
let holdReadback = false;
const makeItem = (
  age: number,
  room = 'book',
  label = 'Publishing assistant'
): ApprovalInboxItem => ({
  roomId: `!${room}:example.test`,
  eventId: `$request-${++sequence}`,
  sender: `@${label.toLowerCase().replaceAll(' ', '-')}:example.test`,
  body: '⚠️ Dangerous command requires approval\n```sh\nrm -rf ./build/preview\n```\nReason: Clear the old preview before rebuilding the children’s book.\n\nReply !approve to execute, !approve always to approve permanently, or !deny to cancel.',
  originServerTs: Date.now() - age,
  expiresAt: Date.now() - age + 300000,
  status: 'pending',
  canSendReaction: !query.has('noPermission'),
  bodyTruncated: query.has('truncated'),
});
if (!query.has('empty')) items = [makeItem(120000), makeItem(30000, 'home', 'Home assistant')];
if (query.has('expires')) {
  items = [makeItem(297000)];
}
if (query.has('unparsed')) {
  items[0].body = '⚠️ Dangerous command requires approval';
  items[1].body = 'Unrecognized approval request: inspect the original operation';
}
Object.assign(window, {
  __TAURI_INTERNALS__: {
    invoke: async (command: string, args?: Record<string, unknown>) => {
      if (command === 'matrix_agent_approvals_list') {
        lastDiscoveryActive = args?.discoveryActive === true;
        if (failed || holdReadback) throw new Error('Fixture offline');
        return {
          sessionGeneration,
          coverageWindowMs: 300000,
          coverage: query.has('latest') || !lastDiscoveryActive ? 'latest_event' : 'discovery',
          items: items.map((item) => ({ ...item })),
          loading,
          incomplete,
        };
      }
      if (command === 'matrix_agent_approval_decide') {
        if (query.has('decisionError')) throw new Error('Fixture rejection');
        const item = items.find(
          (value) => value.eventId === args?.eventId && value.roomId === args?.roomId
        );
        if (!item) throw new Error('Fixture missing request');
        if (query.has('delayedDecision'))
          await new Promise<void>((resolve) => {
            finishDecision = resolve;
          });
        decisions += 1;
        lastAction = String(args?.actionId);
        item.status = 'decided';
        return { roomId: item.roomId, eventId: item.eventId, status: 'applied' };
      }
      if (command === 'matrix_agent_question_answer') {
        if (query.has('questionError')) throw new Error('Fixture rejection');
        questionAnswers.push(args ?? {});
        const answer = args?.answer as { kind?: string } | undefined;
        return {
          roomId: args?.roomId,
          eventId: args?.eventId,
          status: 'sent',
          route:
            String(args?.eventId).startsWith('$structured') &&
            (answer?.kind === 'choice' || answer?.kind === 'skip')
              ? 'reaction'
              : 'reply',
        };
      }
      if (command === 'desktop_spellcheck_enable') return true;
      throw new Error(`Unexpected fixture command: ${command}`);
    },
  },
});
const isLight = query.has('light');
document.body.classList.add(
  configClass,
  varsClass,
  isLight ? synaraLightTheme : darkTheme,
  isLight ? 'light-theme' : 'dark-theme'
);
document.body.style.background = color.Background.Container;
document.body.style.color = color.Background.OnContainer;

function ComposerPreview() {
  const editor = useEditor();
  // Browser specs drive and inspect the Slate document through this handle.
  Object.assign(window, { synaraComposerEditor: editor });
  const [toolbar, setToolbar] = useState(query.has('toolbar'));
  return (
    <>
      <Text size="H5">Composer preview</Text>
      <div data-testid="composer" style={{ marginTop: 16 }}>
        <RoomComposer
          editor={editor}
          placeholder="Send a message..."
          leadingAction={
            <IconButton
              className={composerCss.ComposerAction}
              fill="None"
              aria-label="More message actions"
              variant="Surface"
              size="300"
              radii="Pill"
            >
              <Icon src={Icons.Plus} size="100" />
            </IconButton>
          }
          floatingActions={
            <>
              <IconButton
                className={composerCss.ComposerAction}
                fill="None"
                aria-label="Formatting"
                variant="SurfaceVariant"
                size="300"
                radii="300"
                aria-pressed={toolbar}
                aria-expanded={toolbar}
                onClick={() => setToolbar(!toolbar)}
              >
                <Icon src={toolbar ? Icons.AlphabetUnderline : Icons.Alphabet} />
              </IconButton>
              <IconButton
                className={composerCss.ComposerAction}
                fill="None"
                aria-label="Emoji"
                variant="SurfaceVariant"
                size="300"
                radii="300"
              >
                <Icon src={Icons.Smile} />
              </IconButton>
              <IconButton
                className={composerCss.ComposerAction}
                fill="None"
                aria-label="Send preview"
                variant="Primary"
                size="300"
                radii="300"
              >
                <Icon src={Icons.Send} />
              </IconButton>
            </>
          }
          toolbarVisible={toolbar}
          toolbar={<Toolbar />}
        />
      </div>
    </>
  );
}

const colourChoices = [
  { index: 1, label: 'Red', recommended: true },
  { index: 2, label: 'Blue', recommended: false },
  { index: 3, label: 'Green', recommended: false },
];
const baseQuestion: NativeTimelineAgentQuestion = {
  source: 'structured',
  question: 'Which colour should the cover use?',
  choices: colourChoices,
  multiSelect: false,
  allowOther: true,
  allowSkip: true,
  state: 'pending',
  canAnswer: true,
};
const questionFixtures: Array<{
  name: string;
  eventId: string;
  question: NativeTimelineAgentQuestion;
  ownReplyBody?: string;
}> = [
  { name: 'Single choice', eventId: '$structured-single', question: baseQuestion },
  {
    name: 'Multiple choice',
    eventId: '$structured-multi',
    question: {
      ...baseQuestion,
      question: 'Which formats should I export?',
      choices: [
        { index: 1, label: 'PDF', recommended: true },
        { index: 2, label: 'EPUB', recommended: false },
        { index: 3, label: 'Print', recommended: false },
      ],
      multiSelect: true,
      allowOther: false,
    },
  },
  {
    name: 'Text fallback',
    eventId: '$fallback-card',
    question: {
      ...baseQuestion,
      source: 'fallback',
      question: 'Which database should I use?',
      choices: [
        { index: 1, label: 'Postgres', recommended: true },
        { index: 2, label: 'SQLite', recommended: false },
      ],
    },
  },
  {
    name: 'Open question',
    eventId: '$structured-open',
    question: {
      ...baseQuestion,
      question: 'What should the release be called?',
      choices: [],
      allowSkip: false,
    },
  },
  {
    name: 'Someone else',
    eventId: '$structured-foreign',
    question: { ...baseQuestion, canAnswer: false, requesterUserId: '@alex:example.test' },
  },
  {
    name: 'Settled',
    eventId: '$structured-settled',
    question: { ...baseQuestion, state: 'answered', answer: 'Blue' },
  },
  {
    name: 'Expired',
    eventId: '$structured-expired',
    question: { ...baseQuestion, expiresAtMs: Date.now() - 1 },
  },
  {
    name: 'Answered by reply',
    eventId: '$fallback-replied',
    question: { ...baseQuestion, source: 'fallback' },
    ownReplyBody: '1, 3',
  },
];

function QuestionsPreview() {
  return (
    <main style={{ maxWidth: 860, margin: '32px auto', padding: '0 24px' }}>
      <Text size="H5">Question cards</Text>
      {questionFixtures.map((fixture) => (
        <section
          key={fixture.eventId}
          aria-label={fixture.name}
          data-testid={fixture.eventId}
          style={{ marginTop: 20 }}
        >
          <Text size="L400" priority="300">
            {fixture.name}
          </Text>
          <AgentQuestionCard
            question={fixture.question}
            target={{ roomId: '!book:example.test', eventId: fixture.eventId }}
            ownReplyBody={fixture.ownReplyBody}
          />
        </section>
      ))}
    </main>
  );
}

function Fixture() {
  const { refresh, pendingCount } = useApprovalInbox();
  const navigate = useNavigate();
  const [message, setMessage] = useState('');
  const [, update] = useState(0);
  const change = (run: () => void) => {
    run();
    refresh();
    update((value) => value + 1);
  };
  if (query.has('questions')) return <QuestionsPreview />;
  if (query.has('composer'))
    return (
      <main style={{ maxWidth: 1080, margin: '64px auto', padding: '0 24px' }}>
        <ComposerPreview />
        <p style={{ marginTop: 20, fontSize: 14, opacity: 0.7 }}>
          Type a draft or toggle formatting to review the feel. This preview does not send messages.
        </p>
        <p style={{ marginTop: 16, fontSize: 14 }}>
          <a href="?composer">Dark theme</a> · <a href="?composer&light">Light theme</a>
        </p>
      </main>
    );
  return (
    <>
      <div
        style={{
          padding: 8,
          display: 'flex',
          flexWrap: 'wrap',
          gap: 8,
          fontSize: 12,
          background: color.Surface.Container,
        }}
      >
        <span>Local test fixture</span>
        <button onClick={() => navigate('/home/')}>Leave approvals</button>
        <button onClick={() => navigate('/approvals/')}>Show approvals</button>
        <button
          onClick={() => {
            holdReadback = true;
            const item = items[0];
            void decideAgentApprovalWithNativeOwner({
              roomId: item.roomId,
              eventId: item.eventId,
              actionId: 'agent-approval.deny',
            });
          }}
        >
          Decide outside inbox while readback fails
        </button>
        <output data-testid="discovery-active">{String(lastDiscoveryActive)}</output>
        <button
          onClick={() =>
            change(() => {
              items.push(makeItem(0, 'new', 'Research assistant'));
            })
          }
        >
          New request
        </button>
        <button
          onClick={() =>
            change(() => {
              items.forEach((item) => {
                item.status = 'decided';
              });
            })
          }
        >
          Decision on another device
        </button>
        <button
          onClick={() =>
            change(() => {
              items.forEach((item) => {
                item.expiresAt = Date.now() - 1;
              });
            })
          }
        >
          Expire requests
        </button>
        <button
          onClick={() =>
            change(() => {
              failed = !failed;
            })
          }
        >
          Toggle network failure
        </button>
        <button
          onClick={() =>
            change(() => {
              incomplete = !incomplete;
            })
          }
        >
          Toggle partial coverage
        </button>
        <button
          onClick={() =>
            change(() => {
              sessionGeneration += 1;
              items = items.map((item) => ({ ...item, status: 'pending' }));
            })
          }
        >
          Restart native session
        </button>
        <button
          onClick={() => {
            finishDecision?.();
          }}
        >
          Complete delayed decision
        </button>
        <output data-testid="fixture-decisions">
          {decisions} decisions · {lastAction}
        </output>
        <output data-testid="pending-count">{pendingCount} pending</output>
      </div>
      <div style={{ display: 'flex', height: 'calc(100vh - 62px)', minHeight: 0 }}>
        <Sidebar>
          <SidebarStack>
            <SidebarItem>
              <SidebarAvatar as="button" aria-label="Home" outlined>
                <Icon src={Icons.Home} />
              </SidebarAvatar>
            </SidebarItem>
            <SidebarItem>
              <SidebarAvatar as="button" aria-label="People" outlined>
                <Icon src={Icons.User} />
              </SidebarAvatar>
            </SidebarItem>
            <SidebarStackSeparator />
            <SidebarItem>
              <SidebarAvatar as="button" aria-label="Search" outlined>
                <Icon src={Icons.Search} />
              </SidebarAvatar>
            </SidebarItem>
            <ApprovalsTab />
            <SidebarItem>
              <SidebarAvatar as="button" aria-label="Activity" outlined>
                <Icon src={Icons.Inbox} />
              </SidebarAvatar>
            </SidebarItem>
          </SidebarStack>
        </Sidebar>
        <main
          style={{
            flex: 1,
            minWidth: 0,
            overflowY: 'auto',
            background: color.SurfaceVariant.Container,
          }}
        >
          <ApprovalsView
            roomName={(id) =>
              id.includes('book')
                ? 'Project – Children’s Books'
                : id.includes('home')
                  ? 'Home Assistants'
                  : 'Research'
            }
            senderName={(item) =>
              item.sender.includes('publishing')
                ? 'Publishing assistant'
                : item.sender.includes('home')
                  ? 'Home assistant'
                  : 'Research assistant'
            }
            openMessage={(item) => setMessage(`Opened ${item.roomId} / ${item.eventId}`)}
          />
          {message && (
            <p role="status" style={{ padding: 24 }}>
              {message}
            </p>
          )}
          <div style={{ padding: 24 }}>
            <ComposerPreview />
          </div>
        </main>
      </div>
    </>
  );
}
createRoot(document.getElementById('root')!).render(
  <MemoryRouter initialEntries={[query.has('idle') ? '/home/' : '/approvals/']}>
    <ApprovalInboxProvider>
      <Fixture />
    </ApprovalInboxProvider>
  </MemoryRouter>
);
