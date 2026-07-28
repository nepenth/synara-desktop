import { ClientEvent, ClientEventHandlerMap, MatrixClient } from 'matrix-js-sdk';
import { useEffect } from 'react';

export const useAccountDataCallback = (
  mx: MatrixClient,
  onAccountData: ClientEventHandlerMap[ClientEvent.AccountData],
  enabled = true
) => {
  useEffect(() => {
    if (!enabled) return undefined;
    mx.on(ClientEvent.AccountData, onAccountData);
    return () => {
      mx.removeListener(ClientEvent.AccountData, onAccountData);
    };
  }, [mx, onAccountData, enabled]);
};
