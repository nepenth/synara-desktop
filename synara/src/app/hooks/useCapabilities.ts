import { createContext, useContext } from 'react';

/** Structural projection of the server capabilities object (MatrixClient.getCapabilities). */
export type Capabilities = {
  'm.room_versions'?: { default?: string; available?: Record<string, string> };
  'm.set_avatar_url'?: { enabled?: boolean };
  'm.set_displayname'?: { enabled?: boolean };
};

const CapabilitiesContext = createContext<Capabilities | null>(null);

export const CapabilitiesProvider = CapabilitiesContext.Provider;

export function useCapabilities(): Capabilities {
  const capabilities = useContext(CapabilitiesContext);
  if (!capabilities) throw new Error('Capabilities are not provided!');
  return capabilities;
}
