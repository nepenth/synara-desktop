import { invokeDesktopWithAvailability } from '../../../utils/desktop';

export type NativeIgnoredUsersSnapshot = {
  userIds: string[];
};

const isUserId = (value: string): boolean =>
  value.startsWith('@') && value.includes(':') && value.length > 3;

export async function nativeIgnoredUsersSnapshot(): Promise<string[]> {
  const result = await invokeDesktopWithAvailability<NativeIgnoredUsersSnapshot>(
    'matrix_ignored_users_snapshot'
  );
  if (!result.available || !result.value || !Array.isArray(result.value.userIds)) {
    throw new Error('Native ignored-user list is unavailable.');
  }
  return result.value.userIds.filter(isUserId);
}

export async function nativeIgnoredUsersIgnore(userId: string): Promise<void> {
  const result = await invokeDesktopWithAvailability<{ status: string }>(
    'matrix_ignored_users_ignore',
    { userId }
  );
  if (!result.available || result.value?.status !== 'ok') {
    throw new Error('Native ignore request is unavailable.');
  }
}

export async function nativeIgnoredUsersUnignore(userId: string): Promise<void> {
  const result = await invokeDesktopWithAvailability<{ status: string }>(
    'matrix_ignored_users_unignore',
    { userId }
  );
  if (!result.available || result.value?.status !== 'ok') {
    throw new Error('Native unignore request is unavailable.');
  }
}
