import { useEffect } from 'react';
import { atom, useSetAtom } from 'jotai';
import type { WhatsNewNotes } from '../features/whats-new/whatsNewNotes';

/**
 * The notes to show in the "What's new" card this run, decided once at
 * startup (`WhatsNewStartup`); cleared when the card is dismissed.
 */
export const whatsNewPendingAtom = atom<WhatsNewNotes | undefined>(undefined);

/** How many blocking prompts are open; the card waits while any is. */
export const whatsNewBlockersAtom = atom(0);

/**
 * Keeps the "What's new" card hidden while `active`: for prompts that
 * need the person's attention first, such as device verification.
 */
export const useBlockWhatsNew = (active: boolean): void => {
  const setBlockers = useSetAtom(whatsNewBlockersAtom);
  useEffect(() => {
    if (!active) return undefined;
    setBlockers((count) => count + 1);
    return () => setBlockers((count) => Math.max(0, count - 1));
  }, [active, setBlockers]);
};
