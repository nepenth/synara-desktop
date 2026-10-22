import React, { FormEventHandler, useEffect, useRef, useState } from 'react';
import {
  Box,
  Button,
  color,
  config,
  Header,
  Icon,
  IconButton,
  Icons,
  Overlay,
  OverlayBackdrop,
  OverlayCenter,
  Spinner,
  Text,
} from 'folds';
import { Dialog } from '../../components/modal-dialog';
import FocusTrap from '../../components/FocusTrap';
import { PasswordInput } from '../../components/password-input';
import { NativeSecretStorageGate } from '../../components/SecretStorage';
import { openExternalUrl } from '../../utils/appLinks';
import {
  authenticateIdentityReset,
  cancelIdentityReset,
  continueIdentityReset,
  IDENTITY_RESET_COPY,
  identityResetApprovalUrl,
  identityResetMayLeaveRecoveryOff,
  NativeIdentityResetState,
  startIdentityReset,
} from './nativeIdentityReset';

type ResetStep = 'warning' | 'password' | 'approval' | 'recovery' | 'cancelled';

const nextStep = (state: NativeIdentityResetState): ResetStep => {
  if (state.step === 'password_required') return 'password';
  if (state.step === 'approval_required') return 'approval';
  return 'recovery';
};

/**
 * "Reset Identity": the honest warning, then the password or browser
 * approval the homeserver asks for, then recovery setup for a new key.
 * Cancelling or failing after Start points back to recovery setup: the SDK
 * has already removed key backup and recovery by then.
 */
export function IdentityResetDialog({ onClose }: { onClose: () => void }) {
  const [step, setStep] = useState<ResetStep>('warning');
  const [approvalUrl, setApprovalUrl] = useState<string>();
  const [working, setWorking] = useState(false);
  const [error, setError] = useState<string>();
  const [recoveryMayBeOff, setRecoveryMayBeOff] = useState(false);
  const pending = step === 'password' || step === 'approval';
  const openedApproval = useRef(false);

  useEffect(() => {
    if (step === 'approval' && approvalUrl && !openedApproval.current) {
      openedApproval.current = true;
      void openExternalUrl(approvalUrl);
    }
  }, [step, approvalUrl]);

  const run = async (operation: () => Promise<NativeIdentityResetState>) => {
    setWorking(true);
    setError(undefined);
    try {
      const state = await operation();
      setApprovalUrl(identityResetApprovalUrl(state));
      setStep(nextStep(state));
    } catch (failure) {
      const message = failure instanceof Error ? failure.message : undefined;
      setError(message);
      if (message === undefined || identityResetMayLeaveRecoveryOff(message)) {
        setRecoveryMayBeOff(true);
      }
    } finally {
      setWorking(false);
    }
  };

  const close = () => {
    if (!pending) {
      onClose();
      return;
    }
    setWorking(true);
    void cancelIdentityReset().finally(() => {
      setWorking(false);
      setError(undefined);
      setStep('cancelled');
    });
  };

  const authenticate: FormEventHandler<HTMLFormElement> = (event) => {
    event.preventDefault();
    if (working) return;
    const input = event.currentTarget.elements.namedItem('password') as HTMLInputElement | null;
    const password = input?.value ?? '';
    if (!password) return;
    void run(() => authenticateIdentityReset(password)).finally(() => {
      if (input) input.value = '';
    });
  };

  const errorText = error && (
    <>
      <Text size="T200" style={{ color: color.Critical.Main }}>
        {error}
      </Text>
      {recoveryMayBeOff && <Text size="T200">{IDENTITY_RESET_COPY.recoveryMayBeOff}</Text>}
    </>
  );

  return (
    <Overlay open backdrop={<OverlayBackdrop />}>
      <OverlayCenter>
        <FocusTrap
          focusTrapOptions={{
            initialFocus: false,
            clickOutsideDeactivates: false,
            escapeDeactivates: false,
          }}
        >
          <Dialog role={step === 'warning' ? 'alertdialog' : 'dialog'}>
            <Header
              style={{
                padding: `0 ${config.space.S200} 0 ${config.space.S400}`,
                borderBottomWidth: config.borderWidth.B300,
              }}
              variant="Surface"
              size="500"
            >
              <Box grow="Yes">
                <Text size="H4">{IDENTITY_RESET_COPY.title}</Text>
              </Box>
              <IconButton
                size="300"
                radii="300"
                onClick={close}
                disabled={working}
                aria-label="Close"
              >
                <Icon src={Icons.Cross} />
              </IconButton>
            </Header>
            <Box
              as={step === 'password' ? 'form' : undefined}
              onSubmit={step === 'password' ? authenticate : undefined}
              style={{ padding: config.space.S400 }}
              direction="Column"
              gap="400"
            >
              {step === 'warning' && (
                <>
                  <Text size="T300">{IDENTITY_RESET_COPY.warningIntro}</Text>
                  <Box as="ul" direction="Column" gap="200" style={{ margin: 0, paddingLeft: 20 }}>
                    {IDENTITY_RESET_COPY.warnings.map((warning) => (
                      <Text as="li" size="T300" key={warning}>
                        {warning}
                      </Text>
                    ))}
                  </Box>
                  <Box gap="200">
                    <Button
                      variant="Critical"
                      disabled={working}
                      before={working && <Spinner size="200" variant="Critical" fill="Solid" />}
                      onClick={() => void run(startIdentityReset)}
                    >
                      <Text size="B400">{IDENTITY_RESET_COPY.title}</Text>
                    </Button>
                    <Button fill="Soft" variant="Secondary" disabled={working} onClick={close}>
                      <Text size="B400">Cancel</Text>
                    </Button>
                  </Box>
                </>
              )}
              {step === 'password' && (
                <>
                  <Text size="T300">{IDENTITY_RESET_COPY.password}</Text>
                  <Box direction="Column" gap="100">
                    <Text size="L400">Account Password</Text>
                    <PasswordInput name="password" size="400" readOnly={working} autoFocus />
                  </Box>
                  <Box gap="200">
                    <Button
                      type="submit"
                      variant="Critical"
                      disabled={working}
                      before={working && <Spinner size="200" variant="Critical" fill="Solid" />}
                    >
                      <Text size="B400">Authenticate</Text>
                    </Button>
                    <Button
                      type="button"
                      fill="Soft"
                      variant="Secondary"
                      disabled={working}
                      onClick={close}
                    >
                      <Text size="B400">Cancel</Text>
                    </Button>
                  </Box>
                </>
              )}
              {step === 'approval' && (
                <>
                  <Text size="T300">{IDENTITY_RESET_COPY.approval}</Text>
                  <Box gap="200" wrap="Wrap">
                    {approvalUrl && (
                      <Button
                        fill="Soft"
                        variant="Secondary"
                        disabled={working}
                        onClick={() => void openExternalUrl(approvalUrl)}
                      >
                        <Text size="B400">{IDENTITY_RESET_COPY.openAccountPage}</Text>
                      </Button>
                    )}
                    <Button
                      variant="Critical"
                      disabled={working}
                      before={working && <Spinner size="200" variant="Critical" fill="Solid" />}
                      onClick={() => void run(continueIdentityReset)}
                    >
                      <Text size="B400">Continue</Text>
                    </Button>
                    <Button fill="Soft" variant="Secondary" disabled={working} onClick={close}>
                      <Text size="B400">Cancel</Text>
                    </Button>
                  </Box>
                </>
              )}
              {step === 'cancelled' && (
                <>
                  <Text size="T300">{IDENTITY_RESET_COPY.cancelled}</Text>
                  <Button type="button" onClick={onClose}>
                    <Text size="B400">Close</Text>
                  </Button>
                </>
              )}
              {step === 'recovery' && (
                <>
                  <Text size="T300">{IDENTITY_RESET_COPY.recovery}</Text>
                  <NativeSecretStorageGate>
                    {() => (
                      <>
                        <Text size="T300">
                          Device verification and account recovery are configured for this session.
                        </Text>
                        <Button type="button" onClick={onClose}>
                          <Text size="B400">Done</Text>
                        </Button>
                      </>
                    )}
                  </NativeSecretStorageGate>
                </>
              )}
              {errorText}
            </Box>
          </Dialog>
        </FocusTrap>
      </OverlayCenter>
    </Overlay>
  );
}

/** Security settings entry: opens the reset dialog. */
export function IdentityResetButton() {
  const [open, setOpen] = useState(false);
  return (
    <>
      <Button size="300" radii="300" variant="Critical" fill="Soft" onClick={() => setOpen(true)}>
        <Text as="span" size="B300">
          {IDENTITY_RESET_COPY.title}
        </Text>
      </Button>
      {open && <IdentityResetDialog onClose={() => setOpen(false)} />}
    </>
  );
}
