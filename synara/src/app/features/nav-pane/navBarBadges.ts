/**
 * What each bottom-bar item says beside its icon. These are the rail's badge
 * rules, moved unchanged (review-2): a red count only for mentions, unread
 * DMs, invites, overdue Later items and pending approvals; a dot for plain
 * unread.
 */

export type BarBadge =
  | { kind: 'none' }
  /** The 8px unread dot (neutral). */
  | { kind: 'dot' }
  | { kind: 'count'; count: number; highlight: boolean }
  /** A solid status pill: approvals with partial coverage, unverified devices. */
  | { kind: 'status'; text: string; tone: 'Success' | 'Warning' | 'Critical' };

const NONE: BarBadge = { kind: 'none' };

/**
 * Home: one list of rooms and DMs (NAV-12), so one attention badge. A red
 * count for room mentions plus unread DM messages, a dot for plain unread.
 */
export const homeBarBadge = (unread: boolean, attention: number): BarBadge => {
  if (!unread) return NONE;
  return attention > 0 ? { kind: 'count', count: attention, highlight: true } : { kind: 'dot' };
};

export type LaterSummary = { active: number; overdue: number };

/** Activity: invites plus active Later items; red when an invite or overdue item waits. */
export const activityBarBadge = (inviteCount: number, later: LaterSummary): BarBadge => {
  const count = inviteCount + later.active;
  if (count <= 0) return NONE;
  return { kind: 'count', count, highlight: inviteCount > 0 || later.overdue > 0 };
};

export type ActivityPage = 'invites' | 'later' | 'notifications';

/** Where Activity opens with no remembered page: invites, then Later, then notifications. */
export const activityLandingPage = (inviteCount: number, laterActive: number): ActivityPage => {
  if (inviteCount > 0) return 'invites';
  if (laterActive > 0) return 'later';
  return 'notifications';
};

export type ApprovalsSummaryInput = {
  pendingCount: number;
  loading: boolean;
  incomplete: boolean;
  coverage?: string;
  error?: unknown;
};

export type ApprovalsBarState = {
  /** Accessible name and tooltip: "Approvals · 2 pending · recent activity". */
  label: string;
  /** Why the count may be partial; the badge's `title`. */
  coverageDescription: string;
  badge: BarBadge;
};

export const approvalsBarState = ({
  pendingCount,
  loading,
  incomplete,
  coverage,
  error,
}: ApprovalsSummaryInput): ApprovalsBarState => {
  const needsAttention = incomplete || Boolean(error);
  const coverageDescription = error
    ? 'unavailable'
    : incomplete
      ? 'partial coverage'
      : loading
        ? 'checking rooms'
        : coverage === 'latest_event'
          ? 'recent activity'
          : '';
  const label = `Approvals${
    pendingCount ? ` · ${pendingCount}${needsAttention ? '+' : ''} pending` : ''
  }${coverageDescription ? ` · ${coverageDescription}` : ''}`;
  let badge: BarBadge = NONE;
  if (needsAttention) {
    badge =
      pendingCount > 0
        ? { kind: 'status', text: `${pendingCount}+`, tone: 'Success' }
        : { kind: 'status', text: '!', tone: 'Warning' };
  } else if (pendingCount > 0) {
    badge = { kind: 'count', count: pendingCount, highlight: true };
  }
  return { label, coverageDescription, badge };
};

export type AccountBarState = {
  label: string;
  badge: BarBadge;
  /** The account menu's verification entry, when one is needed. */
  verify?: 'this-device' | 'other-devices';
};

/**
 * The account avatar carries the former rail's unverified warning: Critical
 * when this device is unverified, a Warning count for other unverified
 * devices. Nothing while verification cannot be offered (no cross-signing).
 */
export const accountBarState = (
  canOfferVerification: boolean,
  ownUnverified: boolean,
  otherUnverifiedCount: number
): AccountBarState => {
  if (!canOfferVerification) return { label: 'Account', badge: NONE };
  if (ownUnverified) {
    return {
      label: 'Account, this device is not verified',
      badge: { kind: 'status', text: '!', tone: 'Critical' },
      verify: 'this-device',
    };
  }
  if (otherUnverifiedCount > 0) {
    return {
      label: `Account, ${otherUnverifiedCount} unverified ${
        otherUnverifiedCount === 1 ? 'device' : 'devices'
      }`,
      badge: { kind: 'status', text: String(otherUnverifiedCount), tone: 'Warning' },
      verify: 'other-devices',
    };
  }
  return { label: 'Account', badge: NONE };
};

/** Bar items show their text labels once the list column is wider than this. */
export const BAR_LABEL_MIN_WIDTH = 300;

/** `width` undefined: the column fills the window (Mobile), so labels fit. */
export const barShowsLabels = (width: number | undefined): boolean =>
  width === undefined || width > BAR_LABEL_MIN_WIDTH;
