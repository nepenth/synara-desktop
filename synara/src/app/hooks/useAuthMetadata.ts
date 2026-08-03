import { createContext, useContext } from 'react';

/** OIDC metadata consumed by the device-management dashboard route. */
export type AuthMetadata = Readonly<{
  account_management_uri?: string;
  issuer?: string;
}>;

const AuthMetadataContext = createContext<AuthMetadata | undefined>(undefined);

export const AuthMetadataProvider = AuthMetadataContext.Provider;

export const useAuthMetadata = (): AuthMetadata | undefined => {
  const metadata = useContext(AuthMetadataContext);

  return metadata;
};
