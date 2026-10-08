import { ReactNode, useCallback } from 'react';

import type { NativeRoomListSnapshot, NativeSessionSnapshot } from '../../state/room-list/roomList';
import { useBindAtoms } from '../../state/hooks/useBindAtoms';

import { nativeSession } from '../../native/nativeSession';
type ClientBindAtomsProps = {
  children: ReactNode;
};
export function ClientBindAtoms({ children }: ClientBindAtomsProps) {
  const applyRoomListSnapshot = useCallback((snapshot: NativeRoomListSnapshot) => {
    nativeSession().applyRoomListSnapshot(snapshot);
  }, []);
  const applyNativeSessionSnapshot = useCallback((snapshot: NativeSessionSnapshot) => {
    nativeSession().applySessionSnapshot(snapshot);
  }, []);
  useBindAtoms(applyRoomListSnapshot, applyNativeSessionSnapshot);

  return children;
}
