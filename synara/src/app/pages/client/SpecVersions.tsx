import React, { ReactNode } from 'react';
import { SpecVersionsLoader } from '../../components/SpecVersionsLoader';
import { SpecVersionsProvider } from '../../hooks/useSpecVersions';

export function SpecVersions({ baseUrl, children }: { baseUrl: string; children: ReactNode }) {
  return (
    // Native sync owns signed-in connection readiness. Optional renderer
    // metadata must not hide the client or its actual recovery controls.
    <SpecVersionsLoader baseUrl={baseUrl} blocking={false}>
      {(versions) => <SpecVersionsProvider value={versions}>{children}</SpecVersionsProvider>}
    </SpecVersionsLoader>
  );
}
