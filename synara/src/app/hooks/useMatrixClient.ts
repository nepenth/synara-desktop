import { createContext, useContext } from 'react';
import type { initClient } from '../../client/initMatrix';

type MatrixClient = Awaited<ReturnType<typeof initClient>>;

const MatrixClientContext = createContext<MatrixClient | null>(null);

export const MatrixClientProvider = MatrixClientContext.Provider;

export function useMaybeMatrixClient(): MatrixClient | null {
  return useContext(MatrixClientContext);
}

export function useMatrixClient(): MatrixClient {
  const mx = useMaybeMatrixClient();
  if (!mx) throw new Error('MatrixClient not initialized!');
  return mx;
}
