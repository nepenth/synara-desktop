import React, {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import { useMatrixClient } from '../../hooks/useMatrixClient';
import {
  approvalIdentity,
  approvalStatus,
  loadApprovalInbox,
  type ApprovalInboxItem,
  type ApprovalInboxSnapshot,
} from './nativeApprovalInbox';

export type ApprovalInboxContextValue = {
  sessionGeneration?: number;
  items: ApprovalInboxItem[];
  pendingCount: number;
  loading: boolean;
  incomplete: boolean;
  error?: string;
  now: number;
  refresh: () => void;
  decided: (item: ApprovalInboxItem) => boolean;
};

export const ApprovalInboxContext = createContext<ApprovalInboxContextValue | undefined>(undefined);

export function ApprovalInboxProvider({ children }: { children: React.ReactNode }) {
  const mx = useMatrixClient();
  const [snapshot, setSnapshot] = useState<ApprovalInboxSnapshot>();
  const [error, setError] = useState<string>();
  const [now, setNow] = useState(Date.now);
  const reload = useRef<() => void>(() => undefined);
  const refresh = useCallback(() => reload.current(), []);
  const completed = useRef(new Set<string>());
  const generation = useRef<number | undefined>(undefined);
  const client = useRef(mx);

  useEffect(() => {
    let cancelled = false;
    let busy = false;
    let queued = false;
    setSnapshot(undefined);
    setError(undefined);
    completed.current.clear();
    generation.current = undefined;
    client.current = mx;
    const load = async () => {
      if (cancelled) return;
      if (busy) {
        queued = true;
        return;
      }
      busy = true;
      try {
        const next = await loadApprovalInbox();
        if (!cancelled) {
          if (generation.current !== next.sessionGeneration) {
            completed.current.clear();
            generation.current = next.sessionGeneration;
          }
          setSnapshot(next);
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
    };
    void load();
    const poll = window.setInterval(() => {
      void load();
    }, 5000);
    const clock = window.setInterval(() => setNow(Date.now()), 1000);
    const focus = () => {
      setNow(Date.now());
      void load();
    };
    window.addEventListener('focus', focus);
    return () => {
      cancelled = true;
      reload.current = () => undefined;
      window.clearInterval(poll);
      window.clearInterval(clock);
      window.removeEventListener('focus', focus);
    };
  }, [mx]);

  const decided = useCallback(
    (item: ApprovalInboxItem) => {
      if (
        client.current !== mx ||
        snapshot?.sessionGeneration === undefined ||
        generation.current !== snapshot.sessionGeneration
      )
        return false;
      completed.current.add(approvalIdentity(item));
      setSnapshot((previous) =>
        previous && previous.sessionGeneration === snapshot.sessionGeneration
          ? {
              ...previous,
              items: previous.items.map((candidate) =>
                approvalIdentity(candidate) === approvalIdentity(item)
                  ? { ...candidate, status: 'decided' }
                  : candidate
              ),
            }
          : previous
      );
      refresh();
      return true;
    },
    [refresh, mx, snapshot?.sessionGeneration]
  );

  const value = useMemo<ApprovalInboxContextValue>(() => {
    const items = (snapshot?.items ?? []).map(
      (item): ApprovalInboxItem => ({
        ...item,
        status: completed.current.has(approvalIdentity(item))
          ? 'decided'
          : approvalStatus(item, now),
      })
    );
    return {
      sessionGeneration: snapshot?.sessionGeneration,
      items,
      pendingCount: items.filter((item) => item.status === 'pending').length,
      loading: (!snapshot && !error) || Boolean(snapshot?.loading),
      incomplete: Boolean(snapshot?.incomplete),
      error,
      now,
      refresh,
      decided,
    };
  }, [snapshot, error, now, refresh, decided]);

  return <ApprovalInboxContext.Provider value={value}>{children}</ApprovalInboxContext.Provider>;
}

export function useApprovalInbox() {
  const value = useContext(ApprovalInboxContext);
  if (!value) throw new Error('ApprovalInboxProvider is missing.');
  return value;
}
