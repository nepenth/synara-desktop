import { ReactNode, useCallback } from 'react';

import { useMatrixClient } from '../../hooks/useMatrixClient';
import type { NativeRoomListSnapshot, NativeSessionSnapshot } from '../../state/room-list/roomList';
import { useBindAtoms } from '../../state/hooks/useBindAtoms';

type ClientBindAtomsProps = {
  children: ReactNode;
};
export function ClientBindAtoms({ children }: ClientBindAtomsProps) {
  const mx = useMatrixClient();
  const applyRoomListSnapshot = useCallback(
    (snapshot: NativeRoomListSnapshot) => {
      mx.applyRoomListSnapshot(snapshot);
    },
    [mx]
  );
  const applyNativeSessionSnapshot = useCallback(
    (snapshot: NativeSessionSnapshot) => {
      mx.applyNativeSessionSnapshot(snapshot);
    },
    [mx]
  );
  useBindAtoms(applyRoomListSnapshot, applyNativeSessionSnapshot);

  return children;
}
