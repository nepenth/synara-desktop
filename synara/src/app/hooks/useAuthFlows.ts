import { createContext, useContext } from 'react';
import type { LoginFlowsDto } from '../pages/auth/login/nativeLoginFlows';
import type { UIAAuthData } from '../utils/matrix-uia';

export enum RegisterFlowStatus {
  FlowRequired = 401,
  InvalidRequest = 400,
  RegistrationDisabled = 403,
  RateLimited = 429,
}

/**
 * Stub register-flows shape retained on the shared AuthFlows context for login.
 * Product registration discovery is native-owned by V-AUTH.4b (`Register.tsx` /
 * `matrix_register_flows`) and is not loaded via this context.
 */
export type RegisterFlowsResponse =
  | {
      status: RegisterFlowStatus.FlowRequired;
      data: UIAAuthData;
    }
  | {
      status: Exclude<RegisterFlowStatus, RegisterFlowStatus.FlowRequired>;
    };

export type AuthFlows = {
  loginFlows: LoginFlowsDto;
  registerFlows: RegisterFlowsResponse;
};

const AuthFlowsContext = createContext<AuthFlows | null>(null);

export const AuthFlowsProvider = AuthFlowsContext.Provider;

export const useAuthFlows = (): AuthFlows => {
  const authFlows = useContext(AuthFlowsContext);
  if (!authFlows) {
    throw new Error('Auth Flow info is not loaded!');
  }
  return authFlows;
};
