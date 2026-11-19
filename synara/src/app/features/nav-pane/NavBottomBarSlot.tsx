import React, { useLayoutEffect, useRef } from 'react';
import { createPortal } from 'react-dom';
import { atom, useAtomValue, useSetAtom } from 'jotai';
import { usePageNavWidth } from '../../components/page';
import { NavBottomBar } from './NavBottomBar';

/**
 * The bottom bar belongs to the signed-in client shell, not to a pane.
 *
 * Each pane (Home, Space, Explore, Activity, and Approvals and Create, which
 * borrow the Home list) is its own route element with its own list column,
 * so a bar rendered as a pane's footer was unmounted and mounted again on
 * every move between them: the avatar `<img>` was a new node, the profile
 * was fetched again (its initial showed meanwhile), and the device
 * verification probe started empty, so the warning badge blinked.
 *
 * `NavBottomBarHost` renders the one bar for the client shell into a single
 * element that lives as long as the shell. Each pane renders a
 * `NavBottomBarSlot` where the bar sits (its list column's footer); the slot
 * moves that element into place and reports the column width. Moving a DOM
 * node keeps it, its children and the React state behind them, so the bar
 * keeps its position, landmark and tab order in the column without ever
 * remounting. When no pane shows a bar (a room alone on Mobile), the bar
 * stays mounted off-screen and comes back as it was.
 */

type SlotRegistration = { id: number; width: number | undefined };

/** Mounted slots, most recent last; the last one holds the bar. */
const slotsAtom = atom<SlotRegistration[]>([]);

let barElement: HTMLDivElement | undefined;
const getBarElement = (): HTMLDivElement => {
  if (!barElement) {
    barElement = document.createElement('div');
    // No box of its own: the bar lays out as the column's footer did.
    barElement.style.display = 'contents';
    barElement.setAttribute('data-nav-bottom-bar-host', '');
  }
  return barElement;
};

let nextSlotId = 0;

/** Where a pane's bottom bar goes: the footer of its list column. */
export function NavBottomBarSlot() {
  const slotRef = useRef<HTMLDivElement>(null);
  const idRef = useRef<number | undefined>(undefined);
  if (idRef.current === undefined) {
    nextSlotId += 1;
    idRef.current = nextSlotId;
  }
  const id = idRef.current;
  const width = usePageNavWidth();
  const setSlots = useSetAtom(slotsAtom);

  // Before paint, in the same commit that removed the previous pane's column.
  useLayoutEffect(() => {
    slotRef.current?.appendChild(getBarElement());
    setSlots((slots) => [...slots.filter((slot) => slot.id !== id), { id, width }]);
    return () => {
      setSlots((slots) => slots.filter((slot) => slot.id !== id));
    };
    // Width changes are reported below without moving the bar.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [id, setSlots]);

  useLayoutEffect(() => {
    setSlots((slots) => slots.map((slot) => (slot.id === id ? { ...slot, width } : slot)));
  }, [id, width, setSlots]);

  return <div ref={slotRef} style={{ display: 'contents' }} data-nav-bottom-bar-slot="" />;
}

/**
 * The client shell's one bottom bar. Mount once, inside the router and the
 * client providers, for as long as the user is signed in.
 */
export function NavBottomBarHost() {
  const slots = useAtomValue(slotsAtom);
  const active = slots[slots.length - 1];
  // While no pane shows the bar it keeps the width it last had.
  const lastWidth = useRef<number | undefined>(undefined);
  if (active) lastWidth.current = active.width;
  return createPortal(<NavBottomBar navWidth={lastWidth.current} />, getBarElement());
}
