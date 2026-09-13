// Real provider and native decision owner; only native transport/account identity are fixtures.
import React, { memo, useLayoutEffect, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { MemoryRouter, useNavigate } from 'react-router-dom';
import {
  ApprovalInboxProvider,
  useApprovalInbox,
  useApprovalInboxSummary,
} from '../../src/app/features/approvals/ApprovalInboxProvider';
import type { ApprovalInboxSnapshot } from '../../src/app/features/approvals/nativeApprovalInbox';
import { decideAgentApprovalWithNativeOwner } from '../../src/app/features/room/nativeReactionOwner';
import { MatrixClientProvider } from '../../src/app/hooks/useMatrixClient';

let account = 1;
let heldRead = false;
let finishRead: (() => void) | undefined;
let finishDecision: (() => void) | undefined;
const evidence = {
  reads: [] as { account: number; discovery: boolean }[],
  commits: [] as { account: number; bodies: string[]; pending: number }[],
  summaryCommits: 0,
  detailCommits: 0,
  decisionStarted: false,
  decisionCompleted: false,
  readCompleted: false,
};
function snapshot(generation: number): ApprovalInboxSnapshot {
  return {
    sessionGeneration: generation,
    coverageWindowMs: 300000,
    coverage: 'discovery',
    loading: false,
    incomplete: false,
    items: [
      {
        roomId: '!shared:example.test',
        eventId: '$same-event',
        sender: '@agent:example.test',
        body: `Private account ${generation} request`,
        originServerTs: Date.now() - 1000,
        expiresAt: Date.now() + 299000,
        status: 'pending',
        canSendReaction: true,
        bodyTruncated: false,
      },
    ],
  };
}
Object.assign(window, {
  lifecycleEvidence: evidence,
  __TAURI_INTERNALS__: {
    invoke: async (command: string, args?: Record<string, unknown>) => {
      if (command === 'matrix_agent_approvals_list') {
        const requestedAccount = account;
        evidence.reads.push({
          account: requestedAccount,
          discovery: args?.discoveryActive === true,
        });
        if (heldRead) {
          heldRead = false;
          await new Promise<void>((resolve) => {
            finishRead = resolve;
          });
          evidence.readCompleted = true;
        }
        return snapshot(requestedAccount);
      }
      if (command === 'matrix_agent_approval_decide') {
        evidence.decisionStarted = true;
        await new Promise<void>((resolve) => {
          finishDecision = resolve;
        });
        return { roomId: args?.roomId, eventId: args?.eventId, status: 'applied' };
      }
      throw new Error(`Unexpected fixture command: ${command}`);
    },
  },
});
const clients = [{}, {}] as React.ComponentProps<typeof MatrixClientProvider>['value'][];
const SummaryProbe = memo(() => {
  const summary = useApprovalInboxSummary();
  useLayoutEffect(() => {
    evidence.summaryCommits += 1;
  });
  return <output data-testid="summary-count">{summary.pendingCount}</output>;
});
function DetailProbe({ identity }: { identity: number }) {
  const inbox = useApprovalInbox();
  const navigate = useNavigate();
  useLayoutEffect(() => {
    evidence.detailCommits += 1;
    evidence.commits.push({
      account: identity,
      bodies: inbox.items.map((item) => item.body),
      pending: inbox.pendingCount,
    });
  });
  return (
    <>
      <output data-testid="request">
        {inbox.items.map((item) => `${item.body}: ${item.status}`).join(', ')}
      </output>
      <button onClick={() => navigate('/home/')}>Leave approvals</button>
      <button onClick={() => navigate('/approvals/')}>Show approvals</button>
      <button
        onClick={() => {
          heldRead = true;
          inbox.refresh();
        }}
      >
        Hold next read
      </button>
      <button onClick={() => finishRead?.()}>Complete old read</button>
      <button
        onClick={() => {
          const item = inbox.items[0];
          void decideAgentApprovalWithNativeOwner({
            roomId: item.roomId,
            eventId: item.eventId,
            actionId: 'agent-approval.deny',
          }).then(() => {
            evidence.decisionCompleted = true;
          });
        }}
      >
        Begin delayed decision
      </button>
      <button onClick={() => finishDecision?.()}>Complete old decision</button>
    </>
  );
}
function Fixture() {
  const [identity, setIdentity] = useState(1);
  return (
    <MemoryRouter initialEntries={['/approvals/']}>
      <button
        onClick={() => {
          account = 2;
          setIdentity(2);
        }}
      >
        Replace account
      </button>
      <MatrixClientProvider value={clients[identity - 1]}>
        <ApprovalInboxProvider>
          <SummaryProbe />
          <DetailProbe identity={identity} />
        </ApprovalInboxProvider>
      </MatrixClientProvider>
    </MemoryRouter>
  );
}
createRoot(document.getElementById('root')!).render(<Fixture />);
