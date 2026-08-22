import { invokeDesktopWithAvailability } from '../../../utils/desktop';

export type NativeThreepidSnapshot = {
  emails: Array<{ address: string }>;
};

export type NativeThreepidAddResult = {
  status: 'ok' | 'authenticationRequired' | string;
};

const isEmail = (value: string): boolean => value.includes('@') && value.length > 3;

export async function nativeThreepidSnapshot(): Promise<string[]> {
  const result = await invokeDesktopWithAvailability<NativeThreepidSnapshot>(
    'matrix_threepid_snapshot'
  );
  if (!result.available || !result.value || !Array.isArray(result.value.emails)) {
    throw new Error('Native contact addresses are unavailable.');
  }
  return result.value.emails
    .map((email) => email.address)
    .filter((address): address is string => typeof address === 'string' && isEmail(address));
}

export async function nativeThreepidDelete(address: string): Promise<void> {
  const result = await invokeDesktopWithAvailability<{ status: string }>('matrix_threepid_delete', {
    address,
  });
  if (!result.available || result.value?.status !== 'ok') {
    throw new Error('Native contact-address delete is unavailable.');
  }
}

export async function nativeThreepidRequestEmailToken(email: string): Promise<string> {
  const result = await invokeDesktopWithAvailability<{ sessionId: string }>(
    'matrix_threepid_request_email_token',
    { email }
  );
  if (!result.available || typeof result.value?.sessionId !== 'string') {
    throw new Error('Native email verification is unavailable.');
  }
  return result.value.sessionId;
}

export async function nativeThreepidAddEmail(): Promise<NativeThreepidAddResult> {
  const result = await invokeDesktopWithAvailability<NativeThreepidAddResult>(
    'matrix_threepid_add_email'
  );
  if (!result.available || typeof result.value?.status !== 'string') {
    throw new Error('Native email attach is unavailable.');
  }
  return result.value;
}

export async function nativeThreepidAddEmailPassword(
  password: string
): Promise<NativeThreepidAddResult> {
  const result = await invokeDesktopWithAvailability<NativeThreepidAddResult>(
    'matrix_threepid_add_email_password',
    { password }
  );
  if (!result.available || typeof result.value?.status !== 'string') {
    throw new Error('Native email attach is unavailable.');
  }
  return result.value;
}
