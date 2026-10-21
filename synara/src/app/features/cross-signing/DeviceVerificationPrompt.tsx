import React, { useEffect, useState } from 'react';
import {
  Box,
  Button,
  config,
  Header,
  Icon,
  IconButton,
  Icons,
  Line,
  Overlay,
  OverlayBackdrop,
  OverlayCenter,
  Spinner,
  Text,
} from 'folds';
import { Dialog } from '../../components/modal-dialog';
import FocusTrap from '../../components/FocusTrap';
import { NativeSecretStorageAction } from '../../components/SecretStorage';
import { useNativeSecretStorage } from '../../hooks/useNativeSecretStorage';
import { NativeStartVerification } from '../verification/NativeDeviceVerification';
import { IdentityResetDialog } from './IdentityReset';
import {
  dismissDeviceVerificationPrompt,
  IDENTITY_RESET_COPY,
  shouldShowDeviceVerificationPrompt,
  VERIFY_DEVICE_PROMPT_COPY,
} from './nativeIdentityReset';

function RecoveryKeyStep() {
  const { status, loading, refresh } = useNativeSecretStorage();
  if (loading && !status) return <Spinner size="200" variant="Secondary" />;
  if (status?.unlocked) return <Text size="T300">{VERIFY_DEVICE_PROMPT_COPY.verified}</Text>;
  if (status?.action !== 'unlock_required') {
    return <Text size="T300">{VERIFY_DEVICE_PROMPT_COPY.noRecovery}</Text>;
  }
  return <NativeSecretStorageAction status={status} onComplete={refresh} />;
}

type PromptView = 'hidden' | 'choices' | 'recovery' | 'reset';

/**
 * "Verify This Device", once after signing in on a new device of an account
 * that already has an identity. Core decides when (never for the device that
 * created the identity); any choice or "Later" dismisses it for the session.
 */
export function DeviceVerificationPromptRenderer() {
  const [view, setView] = useState<PromptView>('hidden');

  useEffect(() => {
    let disposed = false;
    shouldShowDeviceVerificationPrompt().then((show) => {
      if (!disposed && show) setView('choices');
    });
    return () => {
      disposed = true;
    };
  }, []);

  const choose = (next: PromptView) => {
    void dismissDeviceVerificationPrompt();
    setView(next);
  };

  if (view === 'hidden') return null;
  if (view === 'reset') return <IdentityResetDialog onClose={() => setView('hidden')} />;

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
          <Dialog>
            <Header
              style={{
                padding: `0 ${config.space.S200} 0 ${config.space.S400}`,
                borderBottomWidth: config.borderWidth.B300,
              }}
              variant="Surface"
              size="500"
            >
              <Box grow="Yes">
                <Text size="H4">{VERIFY_DEVICE_PROMPT_COPY.title}</Text>
              </Box>
              <IconButton
                size="300"
                radii="300"
                onClick={() => choose('hidden')}
                aria-label="Close"
              >
                <Icon src={Icons.Cross} />
              </IconButton>
            </Header>
            <Box style={{ padding: config.space.S400 }} direction="Column" gap="400">
              <Text size="T300">{VERIFY_DEVICE_PROMPT_COPY.description}</Text>
              {view === 'recovery' ? (
                <RecoveryKeyStep />
              ) : (
                <Box direction="Column" gap="200" alignItems="Start">
                  <NativeStartVerification onExit={() => choose('hidden')} />
                  <Button variant="Secondary" fill="Soft" onClick={() => choose('recovery')}>
                    <Text size="B400">{VERIFY_DEVICE_PROMPT_COPY.recoveryKey}</Text>
                  </Button>
                </Box>
              )}
              <Line variant="Surface" size="300" />
              <Box direction="Column" gap="200" alignItems="Start">
                <Text size="T200">{VERIFY_DEVICE_PROMPT_COPY.cantVerify}</Text>
                <Button variant="Critical" fill="Soft" onClick={() => choose('reset')}>
                  <Text size="B400">{IDENTITY_RESET_COPY.title}</Text>
                </Button>
              </Box>
              <Box justifyContent="End">
                <Button variant="Secondary" fill="None" onClick={() => choose('hidden')}>
                  <Text size="B400">
                    {view === 'recovery' ? 'Done' : VERIFY_DEVICE_PROMPT_COPY.later}
                  </Text>
                </Button>
              </Box>
            </Box>
          </Dialog>
        </FocusTrap>
      </OverlayCenter>
    </Overlay>
  );
}
