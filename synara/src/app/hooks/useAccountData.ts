import { useMemo } from 'react';
import type { MatrixEventReading } from '../utils/room';
import { accountDataEventReading } from '../utils/room';
import type { AccountDataEvent } from '../../types/matrix/accountData';
import { useNativeAccountData } from '../native/nativeAccountData';

/** Global account data of one type, live from Core's raw account-data cache. */
export function useAccountData(
  eventType: AccountDataEvent,
  enabled = true
): MatrixEventReading | undefined {
  const content = useNativeAccountData(eventType, undefined, enabled);
  return useMemo(
    () => (content ? accountDataEventReading(eventType, content) : undefined),
    [eventType, content]
  );
}
