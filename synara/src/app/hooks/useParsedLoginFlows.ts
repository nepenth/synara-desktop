import { useMemo } from 'react';
import { getPasswordFlow, type LoginFlowDto } from '../pages/auth/login/nativeLoginFlows';

export type { LoginFlowDto };
export { getPasswordFlow };

export type ParsedLoginFlows = {
  password?: LoginFlowDto;
};

export const useParsedLoginFlows = (loginFlows: LoginFlowDto[]) => {
  const parsedFlow: ParsedLoginFlows = useMemo<ParsedLoginFlows>(
    () => ({
      password: getPasswordFlow(loginFlows),
    }),
    [loginFlows]
  );

  return parsedFlow;
};
