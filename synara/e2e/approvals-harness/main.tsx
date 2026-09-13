// Production approval presenter/provider and composer; only the native transport is a fixture.
import React, { useState } from 'react';
import { createRoot } from 'react-dom/client';
import { MemoryRouter, useNavigate } from 'react-router-dom';
import { decideAgentApprovalWithNativeOwner } from '../../src/app/features/room/nativeReactionOwner';
import { Box, Icon, IconButton, Icons, Text, configClass, varsClass, color } from 'folds';
import 'folds/dist/style.css';
import '@fontsource/inter/variable.css';
import '../../src/index.css';
import { darkTheme, synaraLightTheme } from '../../src/colors.css';
import {
  ApprovalInboxProvider,
  useApprovalInbox,
} from '../../src/app/features/approvals/ApprovalInboxProvider';
import { ApprovalsView } from '../../src/app/features/approvals/Approvals';
import type { ApprovalInboxItem } from '../../src/app/features/approvals/nativeApprovalInbox';
import { ApprovalsTab } from '../../src/app/pages/client/sidebar/ApprovalsTab';
import { MatrixClientProvider } from '../../src/app/hooks/useMatrixClient';
import {
  Sidebar,
  SidebarStack,
  SidebarAvatar,
  SidebarItem,
  SidebarStackSeparator,
} from '../../src/app/components/sidebar';
import { CustomEditor, useEditor } from '../../src/app/components/editor/Editor';
import { Toolbar } from '../../src/app/components/editor/Toolbar';

const query = new URLSearchParams(location.search);
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
const mx = {} as React.ComponentProps<typeof MatrixClientProvider>['value'];
Object.assign(window, {
  __TAURI_INTERNALS__: {
    invoke: async (command: string, args?: Record<string, unknown>) => {
      if (command === 'matrix_agent_approvals_list') {
        lastDiscoveryActive = args?.discoveryActive === true;
        if (failed || holdReadback) throw new Error('Fixture offline');
        return {
          sessionGeneration,
          coverageWindowMs: 300000,
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

function Fixture() {
  const { refresh, pendingCount } = useApprovalInbox();
  const navigate = useNavigate();
  const editor = useEditor();
  const [toolbar, setToolbar] = useState(false);
  const [message, setMessage] = useState('');
  const [, update] = useState(0);
  const change = (run: () => void) => {
    run();
    refresh();
    update((value) => value + 1);
  };
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
              <SidebarAvatar as="button" aria-label="Inbox" outlined>
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
            <Text size="H5">Composer preview</Text>
            <div data-testid="composer" style={{ marginTop: 12 }}>
              <CustomEditor
                editor={editor}
                placeholder="Send a message..."
                before={
                  <IconButton aria-label="Attach" variant="SurfaceVariant" size="300" radii="300">
                    <Icon src={Icons.PlusCircle} />
                  </IconButton>
                }
                after={
                  <>
                    <IconButton
                      aria-label="Formatting"
                      variant="SurfaceVariant"
                      size="300"
                      radii="300"
                      aria-pressed={toolbar}
                      onClick={() => setToolbar(!toolbar)}
                    >
                      <Icon src={Icons.Alphabet} />
                    </IconButton>
                    <IconButton aria-label="Emoji" variant="SurfaceVariant" size="300" radii="300">
                      <Icon src={Icons.Smile} />
                    </IconButton>
                    <IconButton aria-label="Send preview" variant="Primary" size="300" radii="300">
                      <Icon src={Icons.Send} />
                    </IconButton>
                  </>
                }
                bottom={toolbar ? <Toolbar /> : undefined}
              />
            </div>
          </div>
        </main>
      </div>
    </>
  );
}
createRoot(document.getElementById('root')!).render(
  <MemoryRouter initialEntries={['/approvals/']}>
    <MatrixClientProvider value={mx}>
      <ApprovalInboxProvider>
        <Fixture />
      </ApprovalInboxProvider>
    </MatrixClientProvider>
  </MemoryRouter>
);
