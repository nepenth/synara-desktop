/**
 * While any modal dialog is open, the app behind it is `inert`: screen readers
 * and the keyboard cannot reach it. Overlays render into `#portalContainer`, a
 * sibling of `#root`, so dialogs, and menus opened from them, stay reachable.
 * The Linux title bar's window controls stay usable.
 */
const holders = new Set<HTMLElement>();
const inertNodes = new Set<Element>();

const KEEP_INTERACTIVE = '[data-synara-window-drag]';

const appRegions = (): Element[] =>
  Array.from(document.getElementById('root')?.children ?? []).filter(
    (element) => !element.matches(KEEP_INTERACTIVE)
  );

const sync = () => {
  if (holders.size === 0) {
    inertNodes.forEach((element) => element.removeAttribute('inert'));
    inertNodes.clear();
    return;
  }
  appRegions().forEach((element) => {
    // A dialog rendered inside the app (no portal) must not make itself inert.
    if (Array.from(holders).some((dialog) => element.contains(dialog))) return;
    if (element.hasAttribute('inert') && !inertNodes.has(element)) return;
    element.setAttribute('inert', '');
    inertNodes.add(element);
  });
};

/**
 * Releases the holds of dialogs inside `container`. A closing dialog's
 * FocusTrap calls this before it returns focus to the opener in the app,
 * which happens before React unmounts the dialog and runs its cleanup.
 */
export const releaseAppInertWithin = (container: Element) => {
  const before = holders.size;
  holders.forEach((dialog) => {
    if (container.contains(dialog)) holders.delete(dialog);
  });
  if (holders.size !== before) sync();
};

/** Makes the app inert until the returned release function runs. */
export const holdAppInert = (dialog: HTMLElement): (() => void) => {
  holders.add(dialog);
  sync();
  return () => {
    holders.delete(dialog);
    sync();
  };
};
