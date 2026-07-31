import { AuthStageData } from '../../hooks/useUIAFlows';

export type RegisterAuthDict =
  | { type: 'm.login.dummy'; session?: string }
  | { type: 'm.login.terms'; session?: string }
  | { type: 'm.login.registration_token'; token: string; session?: string }
  | { type: 'm.login.recaptcha'; response: string; session?: string }
  | {
      type: 'm.login.email.identity';
      sid: string;
      clientSecret: string;
      session?: string;
    };

export type StageComponentProps = {
  stageData: AuthStageData;
  submitAuthDict: (authDict: RegisterAuthDict) => void;
  onCancel: () => void;
};
