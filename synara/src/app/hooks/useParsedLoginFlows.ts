import { useMemo } from 'react';
import { IPasswordFlow, LoginFlow } from 'matrix-js-sdk/lib/@types/auth';

export const getPasswordFlow = (loginFlows: LoginFlow[]): IPasswordFlow | undefined =>
  loginFlows.find((flow) => flow.type === 'm.login.password') as IPasswordFlow;

export type ParsedLoginFlows = {
  password?: LoginFlow;
};
export const useParsedLoginFlows = (loginFlows: LoginFlow[]) => {
  const parsedFlow: ParsedLoginFlows = useMemo<ParsedLoginFlows>(
    () => ({
      password: getPasswordFlow(loginFlows),
    }),
    [loginFlows]
  );

  return parsedFlow;
};
