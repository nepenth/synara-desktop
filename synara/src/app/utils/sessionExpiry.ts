const SESSION_EXPIRED_KEY = 'synara_session_expired';

export const recordSessionExpiry = (): void => {
  try {
    sessionStorage.setItem(SESSION_EXPIRED_KEY, 'true');
  } catch {
    // Native retirement does not depend on renderer storage availability.
  }
};

export const hasSessionExpiryNotice = (): boolean => {
  try {
    return sessionStorage.getItem(SESSION_EXPIRED_KEY) === 'true';
  } catch {
    return false;
  }
};

export const clearSessionExpiryNotice = (): void => {
  try {
    sessionStorage.removeItem(SESSION_EXPIRED_KEY);
  } catch {
    // A stale explanation must not prevent a successful sign-in.
  }
};
