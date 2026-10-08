import type { MatrixEventReading } from '../utils/room';
import { getAccountData } from '../utils/room';
import type { AccountDataEvent } from '../../types/matrix/accountData';

/**
 * Global account data of one type. Native has no general account-data read
 * (only per-feature owners), so this resolves `undefined` like `getAccountData`.
 */
export function useAccountData(
  eventType: AccountDataEvent,
  enabled = true
): MatrixEventReading | undefined {
  return enabled ? getAccountData(eventType) : undefined;
}
