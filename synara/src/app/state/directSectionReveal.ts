import { atom } from 'jotai';

/**
 * The Direct rail tab is a filter over the one Home list (NAV-12): a click
 * sets this, and the Home pane expands its Direct messages section, scrolls it
 * into view and clears the request. A pending request survives the Home pane
 * mounting after the navigation, so it works from any other pane too.
 */
export const directSectionRevealAtom = atom(false);
