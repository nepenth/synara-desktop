import React, {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import { useMatch } from 'react-router-dom';
import { useMatrixClient } from '../../hooks/useMatrixClient';
import { APPROVALS_PATH } from '../../pages/paths';
import {
  loadApprovalInbox,
  type ApprovalInboxItem,
  type ApprovalInboxSnapshot,
} from './nativeApprovalInbox';
import { createApprovalInboxProjection } from './approvalInboxProjection';
import { unionRecentApprovals } from './approvalHistoryProjection';
import {
  loadAgentApprovalHistory,
  subscribeAgentApprovalHistory,
} from './nativeAgentApprovalHistory';
import type { SynaraAgentApprovalHistoryItem } from '../../../types/matrix/accountData';
import {
  activateApprovalDecisionScope,
  subscribeApprovalDecisions,
} from './approvalDecisionEvents';

type ApprovalInboxSummary = {
  coverage: ApprovalInboxSnapshot['coverage'];
  pendingCount: number;
  loading: boolean;
  incomplete: boolean;
  error?: string;
};
export type ApprovalInboxContextValue = ApprovalInboxSummary & {
  sessionGeneration?: number;
  items: ApprovalInboxItem[];
  recentItems: ApprovalInboxItem[];
  now: number;
  refresh: () => void;
  decided: (item: ApprovalInboxItem) => boolean;
};
export const ApprovalInboxContext = createContext<ApprovalInboxContextValue | undefined>(undefined);
const ApprovalInboxSummaryContext = createContext<ApprovalInboxSummary | undefined>(undefined);

export function ApprovalInboxProvider({ children }: { children: React.ReactNode }) {
  const mx = useMatrixClient();
  const onApprovalsPage = Boolean(useMatch({ path: APPROVALS_PATH, end: false }));
  const [visible, setVisible] = useState(() => document.visibilityState !== 'hidden');
  const projection = useMemo(createApprovalInboxProjection, [mx]);
  const [, setRevision] = useState(0);
  const [error, setError] = useState<string>();
  const [now, setNow] = useState(Date.now);
  const [historyItems, setHistoryItems] = useState<SynaraAgentApprovalHistoryItem[]>([]);
  const reload = useRef<() => void>(() => undefined);
  const discoveryActive = useRef(false);
  const pageActive = useRef(onApprovalsPage);
  const refresh = useCallback(() => reload.current(), []);
  const snapshot = projection.read(now);
  const scope = projection.scope;

  useEffect(() => {
    pageActive.current = onApprovalsPage;
    discoveryActive.current = onApprovalsPage && visible;
    refresh();
  }, [onApprovalsPage, visible, refresh]);

  useEffect(() => {
    let cancelled = false;
    let busy = false;
    let queued = false;
    let releaseScope: (() => void) | undefined;
    projection.reset();
    setHistoryItems([]);
    setRevision((value) => value + 1);
    setError(undefined);
    const loadHistory = async () => {
      try {
        const next = await loadAgentApprovalHistory();
        if (!cancelled) setHistoryItems(next.items);
      } catch {
        if (!cancelled) setHistoryItems([]);
      }
    };
    const load = async () => {
      if (cancelled) return;
      if (busy) {
        queued = true;
        return;
      }
      busy = true;
      try {
        const next = await loadApprovalInbox(undefined, discoveryActive.current);
        if (!cancelled) {
          const previousScope = projection.scope;
          projection.receive(next, mx);
          if (projection.scope !== previousScope) {
            releaseScope?.();
            releaseScope = activateApprovalDecisionScope(projection.scope);
          }
          setRevision((value) => value + 1);
          setNow(Date.now());
          setError(undefined);
        }
      } catch {
        if (!cancelled)
          setError('Approval requests could not be refreshed. Retry to check the latest state.');
      } finally {
        busy = false;
        if (queued && !cancelled) {
          queued = false;
          void load();
        }
      }
    };
    reload.current = () => {
      void load();
      void loadHistory();
    };
    // Every successful shared-native decision (room, OS, or inbox) updates one
    // projection immediately. Old-session replies can neither overlay nor refresh it.
    const unsubscribe = subscribeApprovalDecisions((notice) => {
      if (cancelled || !projection.complete(notice.scope, notice)) return;
      setRevision((value) => value + 1);
      void load();
      void loadHistory();
    });
    let unlistenHistory: (() => void) | undefined;
    void subscribeAgentApprovalHistory(() => {
      void loadHistory();
    }).then((unlisten) => {
      if (cancelled) {
        unlisten();
        return;
      }
      unlistenHistory = unlisten;
    });
    void load();
    void loadHistory();
    const poll = window.setInterval(() => {
      void load();
    }, 5000);
    const focus = () => {
      setNow(Date.now());
      void load();
    };
    const visibility = () => {
      const nextVisible = document.visibilityState !== 'hidden';
      setVisible(nextVisible);
      // Update before invoking, without waiting for the React effect.
      discoveryActive.current = pageActive.current && nextVisible;
      setNow(Date.now());
      void load();
    };
    window.addEventListener('focus', focus);
    document.addEventListener('visibilitychange', visibility);
    return () => {
      cancelled = true;
      releaseScope?.();
      unsubscribe();
      unlistenHistory?.();
      reload.current = () => undefined;
      window.clearInterval(poll);
      window.removeEventListener('focus', focus);
      document.removeEventListener('visibilitychange', visibility);
    };
    // Route changes adjust the discovery ref, not the account's projection.
  }, [mx, projection]);

  const hasPending = snapshot?.items.some((item) => item.status === 'pending') ?? false;
  useEffect(() => {
    if (!hasPending) return undefined;
    const clock = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(clock);
  }, [hasPending]);

  const decided = useCallback(
    (item: ApprovalInboxItem) => {
      if (!projection.complete(scope, item)) return false;
      setRevision((value) => value + 1);
      return true;
    },
    [projection, scope]
  );
  const pendingCount = snapshot?.items.filter((item) => item.status === 'pending').length ?? 0;
  const recentItems = useMemo(
    () => unionRecentApprovals(snapshot?.items ?? [], historyItems),
    [snapshot, historyItems]
  );
  const loading = (!snapshot && !error) || Boolean(snapshot?.loading);
  const incomplete = Boolean(snapshot?.incomplete);
  const coverage = snapshot?.coverage ?? 'latest_event';
  const summary = useMemo(
    () => ({ pendingCount, loading, incomplete, coverage, error }),
    [pendingCount, loading, incomplete, coverage, error]
  );
  const value = useMemo<ApprovalInboxContextValue>(
    () => ({
      ...summary,
      sessionGeneration: snapshot?.sessionGeneration,
      items: snapshot?.items ?? [],
      recentItems,
      now,
      refresh,
      decided,
    }),
    [summary, snapshot, recentItems, now, refresh, decided]
  );

  return (
    <ApprovalInboxSummaryContext.Provider value={summary}>
      <ApprovalInboxContext.Provider value={value}>{children}</ApprovalInboxContext.Provider>
    </ApprovalInboxSummaryContext.Provider>
  );
}

export function useApprovalInbox() {
  const value = useContext(ApprovalInboxContext);
  if (!value) throw new Error('ApprovalInboxProvider is missing.');
  return value;
}
export function useApprovalInboxSummary() {
  const value = useContext(ApprovalInboxSummaryContext);
  if (!value) throw new Error('ApprovalInboxProvider is missing.');
  return value;
}
