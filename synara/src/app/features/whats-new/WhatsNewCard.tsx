import React, { useCallback, useEffect, useId, useRef, useState } from 'react';
import { useAtom, useAtomValue, useSetAtom } from 'jotai';
import { useLocation } from 'react-router-dom';
import { Box, Button, Icon, IconButton, Icons, Text } from 'folds';
import { desktopPlatformSettingsAtom } from '../../state/settings';
import { useSetting } from '../../state/hooks/settings';
import { useAppSettingsOpen } from '../../state/appSettings';
import { whatsNewBlockersAtom, whatsNewPendingAtom } from '../../state/whatsNew';
import { getSessionBootstrapResult } from '../../state/sessionBootstrap';
import { APP_VERSION, openExternalUrlFromClick } from '../../utils/appLinks';
import { BEFORE_QUIT_EVENT } from '../../utils/beforeQuit';
import { SETTINGS_PATH } from '../../pages/paths';
import { BUNDLED_WHATS_NEW } from './bundledNotes';
import { decideWhatsNewAtStartup, type WhatsNewNotes } from './whatsNewNotes';
import * as css from './WhatsNewCard.css';

/** Wait this long after the signed-in window is ready, so the card is not part of its first paint. */
export const WHATS_NEW_SHOW_DELAY_MS = 1500;

/** The headline and items; the card and Settings › About both use it. */
export function WhatsNewItems({ notes }: { notes: WhatsNewNotes }) {
  return (
    <ul className={css.WhatsNewList}>
      {notes.items.map((item) => (
        <li key={item}>
          <Text size="T300">{item}</Text>
        </li>
      ))}
    </ul>
  );
}

/** "Full release notes": the GitHub release, opened in the browser. */
export function WhatsNewReleaseLink({ notes }: { notes: WhatsNewNotes }) {
  return (
    <Button
      as="a"
      href={notes.releaseUrl}
      onClick={(evt) => openExternalUrlFromClick(evt, notes.releaseUrl)}
      rel="noreferrer noopener"
      target="_blank"
      variant="Primary"
      fill="None"
      size="300"
      radii="300"
      after={<Icon src={Icons.External} size="50" />}
      style={{ alignSelf: 'flex-start', paddingLeft: 0 }}
    >
      <Text size="B300">Full release notes</Text>
    </Button>
  );
}

type WhatsNewCardProps = {
  notes: WhatsNewNotes;
  onDismiss: () => void;
};

/**
 * The small card after an update. It does not take focus when it appears;
 * its title and headline are announced politely. Escape inside it, or the
 * close button, dismisses it, and focus goes back to where it came from.
 */
export function WhatsNewCard({ notes, onDismiss }: WhatsNewCardProps) {
  const titleId = useId();
  const cardRef = useRef<HTMLElement>(null);
  // Where focus was before it moved into the card (by Tab or a click).
  const returnFocus = useRef<HTMLElement | null>(null);

  const dismiss = useCallback(() => {
    const card = cardRef.current;
    const target = returnFocus.current;
    const focusInside = !!card && card.contains(document.activeElement);
    onDismiss();
    if (focusInside && target?.isConnected) target.focus();
  }, [onDismiss]);

  // Listeners on the card itself: Escape from any of its controls, and the
  // element focus came from.
  useEffect(() => {
    const card = cardRef.current;
    if (!card) return undefined;
    const onKeyDown = (evt: KeyboardEvent) => {
      if (evt.key !== 'Escape') return;
      evt.stopPropagation();
      dismiss();
    };
    const onFocusIn = (evt: FocusEvent) => {
      const from = evt.relatedTarget;
      if (from instanceof HTMLElement && !card.contains(from)) returnFocus.current = from;
    };
    card.addEventListener('keydown', onKeyDown);
    card.addEventListener('focusin', onFocusIn);
    return () => {
      card.removeEventListener('keydown', onKeyDown);
      card.removeEventListener('focusin', onFocusIn);
    };
  }, [dismiss]);

  return (
    <section
      ref={cardRef}
      className={css.WhatsNewCard}
      aria-labelledby={titleId}
      data-whats-new-card=""
    >
      <Box direction="Column" gap="200">
        <Box alignItems="Start" gap="200">
          <Box grow="Yes" direction="Column" gap="100" role="status" aria-live="polite">
            <Text as="h2" size="H5" id={titleId}>
              What&apos;s new in {notes.version}
            </Text>
            <Text size="T300" priority="400">
              {notes.headline}
            </Text>
          </Box>
          <IconButton
            size="300"
            radii="300"
            variant="SurfaceVariant"
            onClick={dismiss}
            aria-label="Close What's new"
          >
            <Icon src={Icons.Cross} size="100" />
          </IconButton>
        </Box>
        <WhatsNewItems notes={notes} />
        <WhatsNewReleaseLink notes={notes} />
      </Box>
    </section>
  );
}

const useRecordWhatsNewSeen = () => {
  const [, setSeenVersion] = useSetting(desktopPlatformSettingsAtom, 'whatsNewSeenVersion');
  return useCallback(() => setSeenVersion(APP_VERSION), [setSeenVersion]);
};

/**
 * Decides once, when the app starts, whether this run shows "What's new"
 * (see `decideWhatsNewAtStartup`). Mount once at the app root, before
 * sign-in, so a fresh install is recorded before anyone signs in.
 */
export function WhatsNewStartup() {
  const [seenVersion, setSeenVersion] = useSetting(
    desktopPlatformSettingsAtom,
    'whatsNewSeenVersion'
  );
  const setPending = useSetAtom(whatsNewPendingAtom);
  const decided = useRef(false);

  useEffect(() => {
    if (decided.current) return;
    decided.current = true;
    const { session, nativeStoreError } = getSessionBootstrapResult();
    const decision = decideWhatsNewAtStartup({
      seenVersion,
      currentVersion: APP_VERSION,
      session: session ? 'signed-in' : nativeStoreError ? 'unknown' : 'signed-out',
      hasNotes: BUNDLED_WHATS_NEW !== undefined,
    });
    if (decision === 'record') setSeenVersion(APP_VERSION);
    if (decision === 'show') setPending(BUNDLED_WHATS_NEW);
  }, [seenVersion, setSeenVersion, setPending]);

  return null;
}

/**
 * The "What's new" card in the signed-in shell. It waits until the shell
 * has been up for a moment, and stays hidden while Settings or a blocking
 * prompt (device verification) is open. The version is recorded when the
 * card is dismissed, or when the app closes after the card was on screen,
 * so it never comes back for this version; closing before it was seen
 * leaves it for the next start.
 */
export function WhatsNewHost() {
  const [pending, setPending] = useAtom(whatsNewPendingAtom);
  const blockers = useAtomValue(whatsNewBlockersAtom);
  const settingsOpen = useAppSettingsOpen();
  const onSettingsRoute = useLocation().pathname.startsWith(SETTINGS_PATH);
  const recordSeen = useRecordWhatsNewSeen();
  const [ready, setReady] = useState(false);
  const [seen, setSeen] = useState(false);

  useEffect(() => {
    if (!pending) return undefined;
    const timer = window.setTimeout(() => setReady(true), WHATS_NEW_SHOW_DELAY_MS);
    return () => window.clearTimeout(timer);
  }, [pending]);

  const visible = !!pending && ready && blockers === 0 && !settingsOpen && !onSettingsRoute;

  // On screen means drawn in a window that is showing, not one that started hidden.
  useEffect(() => {
    if (!visible || seen) return undefined;
    const check = () => {
      if (document.visibilityState === 'visible') setSeen(true);
    };
    check();
    document.addEventListener('visibilitychange', check);
    return () => document.removeEventListener('visibilitychange', check);
  }, [visible, seen]);

  // Seen and still open when the app quits or the page goes away: record it.
  useEffect(() => {
    if (!seen || !pending) return undefined;
    window.addEventListener(BEFORE_QUIT_EVENT, recordSeen);
    window.addEventListener('pagehide', recordSeen);
    return () => {
      window.removeEventListener(BEFORE_QUIT_EVENT, recordSeen);
      window.removeEventListener('pagehide', recordSeen);
    };
  }, [seen, pending, recordSeen]);

  const dismiss = useCallback(() => {
    recordSeen();
    setPending(undefined);
  }, [recordSeen, setPending]);

  if (!visible || !pending) return null;
  return <WhatsNewCard notes={pending} onDismiss={dismiss} />;
}
