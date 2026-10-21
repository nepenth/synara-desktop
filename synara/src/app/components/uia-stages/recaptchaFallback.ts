import { normalizeSecureHomeserverUrl } from '../../cs-api';
import { AuthStageType } from '../../utils/matrix-uia';

/**
 * The homeserver's own CAPTCHA page for this registration session (Matrix
 * UIA fallback). Google's widget cannot load inside the app: the CSP blocks
 * third-party scripts and frames, and it stays that way.
 */
export const recaptchaFallbackUrl = (
  homeserverBaseUrl: string,
  session: string
): string | undefined => {
  const base = normalizeSecureHomeserverUrl(homeserverBaseUrl);
  if (!base || !session) return undefined;
  return `${base}/_matrix/client/v3/auth/${AuthStageType.Recaptcha}/fallback/web?session=${encodeURIComponent(
    session
  )}`;
};
