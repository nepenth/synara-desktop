import { AFTER_LOGIN_REDIRECT_PATH_KEY } from '../state/sessions';

export const setAfterLoginRedirectPath = (url: string): void => {
  localStorage.setItem(AFTER_LOGIN_REDIRECT_PATH_KEY, url);
};
export const getAfterLoginRedirectPath = (): string | undefined => {
  const url = localStorage.getItem(AFTER_LOGIN_REDIRECT_PATH_KEY);
  return url ?? undefined;
};
export const deleteAfterLoginRedirectPath = (): void => {
  localStorage.removeItem(AFTER_LOGIN_REDIRECT_PATH_KEY);
};
