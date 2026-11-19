/**
 * The App Settings pages. Each value is the page's slug in the settings route
 * (`/settings/<slug>/`) and in `synara://settings/<slug>/` links.
 */
export enum SettingsPages {
  GeneralPage = 'general',
  AppearancePage = 'appearance',
  NotificationPage = 'notifications',
  MessagesPage = 'messages',
  MediaPage = 'media-links',
  PrivacyPage = 'privacy',
  AccountPage = 'account',
  SecurityPage = 'security',
  DevicesPage = 'sessions',
  AdvancedPage = 'advanced',
  DiagnosticsPage = 'diagnostics',
  AboutPage = 'about',
}

const PAGE_SLUGS = new Set<string>(Object.values(SettingsPages));

/** The page a route slug names, or undefined for an unknown slug. */
export const settingsPageFromSlug = (slug: string | undefined): SettingsPages | undefined =>
  slug !== undefined && PAGE_SLUGS.has(slug) ? (slug as SettingsPages) : undefined;

/**
 * The last page viewed in this run of the app, so reopening Settings returns
 * to it. Not stored: a new run starts on General again.
 */
let lastViewedPage: SettingsPages | undefined;

export const lastViewedSettingsPage = (): SettingsPages | undefined => lastViewedPage;

export const rememberSettingsPage = (page: SettingsPages | undefined): void => {
  if (page !== undefined) lastViewedPage = page;
};

/** A place in App Settings: a page, and optionally the row to reveal on it. */
export type SettingsLocation = {
  page: SettingsPages;
  /** The row or section title to scroll to and highlight. */
  setting?: string;
};

/** Where the GIF search switch lives; the composer links here. */
export const GIF_SEARCH_SETTING: SettingsLocation = {
  page: SettingsPages.MediaPage,
  setting: 'GIF search',
};

/**
 * Where the account menu's verification item leads: verifying this session
 * happens on Security & recovery, while other sessions are reviewed and
 * verified from Sessions.
 */
export const verificationSettingsLocation = (
  verify: 'this-device' | 'other-devices'
): SettingsLocation =>
  verify === 'this-device'
    ? { page: SettingsPages.SecurityPage, setting: 'Verify this session' }
    : { page: SettingsPages.DevicesPage, setting: 'Other sessions' };
