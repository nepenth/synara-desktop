import { useEffect, useState } from 'react';
import { invokeDesktopWithAvailability, isSynaraDesktop, listen } from '../utils/desktop';
import {
  createNativeRoomJoinRuleOwner,
  type NativeRoomJoinRuleState,
} from '../features/common-settings/general/nativeRoomJoinRuleOwner';

const initialState = (): NativeRoomJoinRuleState => ({ status: 'loading' });

export const useNativeRoomJoinRule = (roomId: string): NativeRoomJoinRuleState => {
  const [state, setState] = useState<NativeRoomJoinRuleState>(initialState);

  useEffect(() => {
    let active = true;
    let dispose: (() => void) | undefined;
    setState(initialState());
    void createNativeRoomJoinRuleOwner(
      roomId,
      {
        desktopAvailable: isSynaraDesktop(),
        invoke: (command, args) => invokeDesktopWithAvailability(command, args),
        listen,
      },
      (nextState) => {
        if (active) setState(nextState);
      }
    ).then((cleanup) => {
      if (active) dispose = cleanup;
      else cleanup();
    });

    return () => {
      active = false;
      dispose?.();
    };
  }, [roomId]);

  return state;
};
