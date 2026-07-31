import {
  Box,
  Button,
  Checkbox,
  Input,
  Overlay,
  OverlayBackdrop,
  OverlayCenter,
  Spinner,
  Text,
  color,
} from 'folds';
import React, { ChangeEventHandler, useCallback, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { PasswordInput } from '../../../components/password-input';
import {
  AuthStageType,
  getLoginTermUrl,
  getUIAFlowForStages,
  hasStageInFlows,
  requiredStageInFlows,
  type UIAAuthData,
  type UIAFlow,
} from '../../../utils/matrix-uia';
import { useUIACompleted, useUIAFlow, useUIAParams } from '../../../hooks/useUIAFlows';
import { AsyncStatus, useAsyncCallback } from '../../../hooks/useAsyncCallback';
import { useAutoDiscoveryInfo } from '../../../hooks/useAutoDiscoveryInfo';
import { FieldError } from '../FiledError';
import {
  AutoDummyStageDialog,
  AutoTermsStageDialog,
  EmailStageDialog,
  ReCaptchaStageDialog,
  RegistrationTokenStageDialog,
  type RegisterAuthDict,
  type RegisterEmailTokenResult,
} from '../../../components/uia-stages';
import { ConfirmPasswordMatch } from '../../../components/ConfirmPasswordMatch';
import { UIAFlowOverlay } from '../../../components/UIAFlowOverlay';
import { synaraDeviceDisplayName } from '../../../utils/user-agent';
import { openExternalUrlFromClick } from '../../../utils/appLinks';
import {
  deleteAfterLoginRedirectPath,
  getAfterLoginRedirectPath,
} from '../../afterLoginRedirectPath';
import { getHomePath } from '../../pathUtils';
import {
  generateRegisterClientSecret,
  NativeRegisterError,
  RegisterErrorCode,
  requestRegisterEmailToken,
  submitRegister,
  SUPPORTED_REGISTER_STAGES,
  type NativeRegisterAuthStage,
  type NativeRegisterOutcome,
  uiaAuthDataFromChallenge,
} from './nativeRegister';

export { SUPPORTED_REGISTER_STAGES };

type RegisterFormInputs = {
  usernameInput: HTMLInputElement;
  passwordInput: HTMLInputElement;
  confirmPasswordInput: HTMLInputElement;
  tokenInput?: HTMLInputElement;
  emailInput?: HTMLInputElement;
  termsInput?: HTMLInputElement;
};

type FormData = {
  username: string;
  password: string;
  token?: string;
  email?: string;
  terms?: boolean;
  clientSecret: string;
};

const pickStages = (uiaFlows: UIAFlow[], formData: FormData): string[] => {
  const pickedStages: string[] = [];
  if (formData.token) pickedStages.push(AuthStageType.RegistrationToken);
  if (formData.email) pickedStages.push(AuthStageType.Email);
  if (formData.terms) pickedStages.push(AuthStageType.Terms);
  if (hasStageInFlows(uiaFlows, AuthStageType.Recaptcha)) {
    pickedStages.push(AuthStageType.Recaptcha);
  }

  return pickedStages;
};

const toNativeAuthStage = (dict: RegisterAuthDict): NativeRegisterAuthStage => {
  switch (dict.type) {
    case AuthStageType.Dummy:
      return { type: 'dummy', session: dict.session };
    case AuthStageType.Terms:
      return { type: 'terms', session: dict.session };
    case AuthStageType.RegistrationToken:
      return {
        type: 'registration_token',
        token: dict.token,
        session: dict.session,
      };
    case AuthStageType.Recaptcha:
      return {
        type: 'recaptcha',
        response: dict.response,
        session: dict.session,
      };
    case AuthStageType.Email:
      return {
        type: 'email_identity',
        sid: dict.sid,
        clientSecret: dict.clientSecret,
        session: dict.session,
      };
    default:
      return { type: 'session_only' };
  }
};

type RegisterUIAFlowProps = {
  formData: FormData;
  flow: UIAFlow;
  authData: UIAAuthData;
  registerEmailState: ReturnType<
    typeof useAsyncCallback<RegisterEmailTokenResult, Error, [string, string]>
  >[0];
  registerEmail: (email: string, clientSecret: string) => Promise<RegisterEmailTokenResult>;
  onRegisterStage: (dict: RegisterAuthDict) => void;
};
function RegisterUIAFlow({
  formData,
  flow,
  authData,
  registerEmailState,
  registerEmail,
  onRegisterStage,
}: RegisterUIAFlowProps) {
  const completed = useUIACompleted(authData);
  const { getStageToComplete } = useUIAFlow(authData, flow);

  const stageToComplete = getStageToComplete();

  const handleAuthDict = useCallback(
    (authDict: RegisterAuthDict) => {
      onRegisterStage(authDict);
    },
    [onRegisterStage]
  );

  const handleCancel = useCallback(() => {
    window.location.reload();
  }, []);

  if (!stageToComplete) return null;
  return (
    <UIAFlowOverlay
      currentStep={completed.length + 1}
      stepCount={flow.stages.length}
      onCancel={handleCancel}
    >
      {stageToComplete.type === AuthStageType.RegistrationToken && (
        <RegistrationTokenStageDialog
          token={formData.token}
          stageData={stageToComplete}
          submitAuthDict={handleAuthDict}
          onCancel={handleCancel}
        />
      )}
      {stageToComplete.type === AuthStageType.Terms && (
        <AutoTermsStageDialog
          stageData={stageToComplete}
          submitAuthDict={handleAuthDict}
          onCancel={handleCancel}
        />
      )}
      {stageToComplete.type === AuthStageType.Recaptcha && (
        <ReCaptchaStageDialog
          stageData={stageToComplete}
          submitAuthDict={handleAuthDict}
          onCancel={handleCancel}
        />
      )}
      {stageToComplete.type === AuthStageType.Email && (
        <EmailStageDialog
          email={formData.email}
          clientSecret={formData.clientSecret}
          stageData={stageToComplete}
          requestEmailToken={registerEmail}
          emailTokenState={registerEmailState}
          submitAuthDict={handleAuthDict}
          onCancel={handleCancel}
        />
      )}
      {stageToComplete.type === AuthStageType.Dummy && (
        <AutoDummyStageDialog
          stageData={stageToComplete}
          submitAuthDict={handleAuthDict}
          onCancel={handleCancel}
        />
      )}
    </UIAFlowOverlay>
  );
}

type PasswordRegisterFormProps = {
  authData: UIAAuthData;
  uiaFlows: UIAFlow[];
  defaultUsername?: string;
  defaultEmail?: string;
  defaultRegisterToken?: string;
};
export function PasswordRegisterForm({
  authData,
  uiaFlows,
  defaultUsername,
  defaultEmail,
  defaultRegisterToken,
}: PasswordRegisterFormProps) {
  const navigate = useNavigate();
  const serverDiscovery = useAutoDiscoveryInfo();
  const baseUrl = serverDiscovery['m.homeserver'].base_url;
  const params = useUIAParams(authData);
  const termUrl = getLoginTermUrl(params);
  const [formData, setFormData] = useState<FormData>();
  const [ongoingFlow, setOngoingFlow] = useState<UIAFlow>();
  const [ongoingAuthData, setOngoingAuthData] = useState<UIAAuthData>();

  const [registerEmailState, registerEmail] = useAsyncCallback<
    RegisterEmailTokenResult,
    Error,
    [string, string]
  >(
    useCallback(
      async (email, clientSecret) => {
        const result = await requestRegisterEmailToken(baseUrl, email, clientSecret, 1);
        return {
          email,
          clientSecret,
          sid: result.sid,
        };
      },
      [baseUrl]
    )
  );

  const [registerState, handleRegister] = useAsyncCallback<
    NativeRegisterOutcome,
    NativeRegisterError,
    [FormData, NativeRegisterAuthStage]
  >(
    useCallback(
      async (data, auth) =>
        submitRegister(baseUrl, data.username, data.password, auth, synaraDeviceDisplayName()),
      [baseUrl]
    )
  );

  const registerError =
    registerState.status === AsyncStatus.Error ? registerState.error : undefined;

  useEffect(() => {
    if (registerState.status !== AsyncStatus.Success) return;
    const outcome = registerState.data;
    if (outcome.status === 'complete') {
      const afterLoginRedirectPath = getAfterLoginRedirectPath();
      deleteAfterLoginRedirectPath();
      navigate(afterLoginRedirectPath ?? getHomePath(), { replace: true });
      return;
    }
    setOngoingAuthData(uiaAuthDataFromChallenge(outcome));
  }, [registerState, navigate]);

  const handleSubmit: ChangeEventHandler<HTMLFormElement> = (evt) => {
    evt.preventDefault();
    const {
      usernameInput,
      passwordInput,
      confirmPasswordInput,
      emailInput,
      tokenInput,
      termsInput,
    } = evt.target as HTMLFormElement & RegisterFormInputs;
    const token = tokenInput?.value.trim();
    const username = usernameInput.value.trim();
    const password = passwordInput.value;
    const confirmPassword = confirmPasswordInput.value;
    if (password !== confirmPassword) {
      return;
    }
    const email = emailInput?.value.trim();
    const terms = termsInput?.value === 'on';

    if (!username) {
      usernameInput.focus();
      return;
    }

    const fData: FormData = {
      username,
      password,
      token,
      email,
      terms,
      clientSecret: generateRegisterClientSecret(),
    };
    const pickedStages = pickStages(uiaFlows, fData);
    const pickedFlow = getUIAFlowForStages(uiaFlows, pickedStages);
    setOngoingFlow(pickedFlow);
    setFormData(fData);
    setOngoingAuthData(undefined);
    handleRegister(fData, {
      type: 'session_only',
      session: authData.session,
    });
  };

  const handleStage = useCallback(
    (dict: RegisterAuthDict) => {
      if (!formData) return;
      handleRegister(formData, toNativeAuthStage(dict));
    },
    [formData, handleRegister]
  );

  const activeAuthData = ongoingAuthData ?? authData;
  const showUia =
    Boolean(formData) &&
    Boolean(ongoingFlow) &&
    Boolean(ongoingAuthData) &&
    registerState.status !== AsyncStatus.Loading;

  return (
    <>
      <Box as="form" onSubmit={handleSubmit} direction="Inherit" gap="400">
        <Box direction="Column" gap="100">
          <Text as="label" size="L400" priority="300">
            Username
          </Text>
          <Input
            variant="Background"
            defaultValue={defaultUsername}
            name="usernameInput"
            size="500"
            outlined
            required
          />
          {registerError?.code === RegisterErrorCode.UserTaken && (
            <FieldError message="This username is already taken." />
          )}
          {registerError?.code === RegisterErrorCode.UserInvalid && (
            <FieldError message="This username contains invalid characters." />
          )}
          {registerError?.code === RegisterErrorCode.UserExclusive && (
            <FieldError message="This username is reserved." />
          )}
        </Box>
        <ConfirmPasswordMatch initialValue>
          {(match, doMatch, passRef, confPassRef) => (
            <>
              <Box direction="Column" gap="100">
                <Text as="label" size="L400" priority="300">
                  Password
                </Text>
                <PasswordInput
                  ref={passRef}
                  onChange={doMatch}
                  name="passwordInput"
                  variant="Background"
                  size="500"
                  outlined
                  required
                />
                {registerError?.code === RegisterErrorCode.PasswordWeak && (
                  <FieldError
                    message={
                      registerError.message ||
                      'Weak Password. Password rejected by server please choosing more strong Password.'
                    }
                  />
                )}
                {registerError?.code === RegisterErrorCode.PasswordShort && (
                  <FieldError
                    message={
                      registerError.message ||
                      'Short Password. Password rejected by server please choosing more long Password.'
                    }
                  />
                )}
              </Box>
              <Box direction="Column" gap="100">
                <Text as="label" size="L400" priority="300">
                  Confirm Password
                </Text>
                <PasswordInput
                  ref={confPassRef}
                  onChange={doMatch}
                  name="confirmPasswordInput"
                  variant="Background"
                  size="500"
                  style={{ color: match ? undefined : color.Critical.Main }}
                  outlined
                  required
                />
              </Box>
            </>
          )}
        </ConfirmPasswordMatch>
        {hasStageInFlows(uiaFlows, AuthStageType.RegistrationToken) && (
          <Box direction="Column" gap="100">
            <Text as="label" size="L400" priority="300">
              {requiredStageInFlows(uiaFlows, AuthStageType.RegistrationToken)
                ? 'Registration Token'
                : 'Registration Token (Optional)'}
            </Text>
            <Input
              variant="Background"
              defaultValue={defaultRegisterToken}
              name="tokenInput"
              size="500"
              required={requiredStageInFlows(uiaFlows, AuthStageType.RegistrationToken)}
              outlined
            />
          </Box>
        )}
        {hasStageInFlows(uiaFlows, AuthStageType.Email) && (
          <Box direction="Column" gap="100">
            <Text as="label" size="L400" priority="300">
              {requiredStageInFlows(uiaFlows, AuthStageType.Email) ? 'Email' : 'Email (Optional)'}
            </Text>
            <Input
              variant="Background"
              defaultValue={defaultEmail}
              name="emailInput"
              type="email"
              size="500"
              required={requiredStageInFlows(uiaFlows, AuthStageType.Email)}
              outlined
            />
          </Box>
        )}

        {hasStageInFlows(uiaFlows, AuthStageType.Terms) && termUrl && (
          <Box alignItems="Center" gap="200">
            <Checkbox name="termsInput" size="300" variant="Primary" required />
            <Text size="T300">
              I accept server{' '}
              <a
                href={termUrl}
                target="_blank"
                rel="noreferrer"
                onClick={(evt) => openExternalUrlFromClick(evt, termUrl)}
              >
                Terms and Conditions
              </a>
              .
            </Text>
          </Box>
        )}
        {registerError?.code === RegisterErrorCode.RateLimited && (
          <FieldError message="Failed to register. Your register request has been rate-limited by server, Please try after some time." />
        )}
        {registerError?.code === RegisterErrorCode.Forbidden && (
          <FieldError message="Failed to register. The homeserver does not permit registration." />
        )}
        {registerError?.code === RegisterErrorCode.InvalidRequest && (
          <FieldError message="Failed to register. Invalid request." />
        )}
        {registerError?.code === RegisterErrorCode.Unsupported && (
          <FieldError message="Failed to register. This application does not support a required authentication stage." />
        )}
        {registerError?.code === RegisterErrorCode.Unknown && (
          <FieldError message={registerError.message || 'Failed to register. Unknown Reason.'} />
        )}
        <span data-spacing-node />
        <Button variant="Primary" size="500" type="submit">
          <Text as="span" size="B500">
            Register
          </Text>
        </Button>
      </Box>
      {showUia && formData && ongoingFlow && ongoingAuthData && (
        <RegisterUIAFlow
          formData={formData}
          flow={ongoingFlow}
          authData={activeAuthData}
          registerEmail={registerEmail}
          registerEmailState={registerEmailState}
          onRegisterStage={handleStage}
        />
      )}
      {registerState.status === AsyncStatus.Loading && (
        <Overlay open backdrop={<OverlayBackdrop />}>
          <OverlayCenter>
            <Spinner variant="Secondary" size="600" />
          </OverlayCenter>
        </Overlay>
      )}
    </>
  );
}
