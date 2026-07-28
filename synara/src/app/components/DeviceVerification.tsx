import {
  ShowSasCallbacks,
  VerificationPhase,
  VerificationRequest,
  Verifier,
} from 'matrix-js-sdk/lib/crypto-api';
import React, { CSSProperties, useCallback, useEffect, useMemo, useState } from 'react';
import { VerificationMethod } from 'matrix-js-sdk/lib/types';
import {
  Box,
  Button,
  config,
  Dialog,
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
import FocusTrap from 'focus-trap-react';
import {
  useVerificationRequestPhase,
  useVerifierCancel,
  useVerifierShowSas,
} from '../hooks/useVerificationRequest';
import { AsyncStatus, useAsyncCallback } from '../hooks/useAsyncCallback';
import { ContainerColor } from '../styles/ContainerColor.css';
import {
  cancelVerificationRequestForExit,
  getInitialSasCallbacks,
  phaseFromVerifierCancellation,
  verificationErrorMessage,
} from '../utils/verification';
import { useMatrixClient } from '../hooks/useMatrixClient';
import { ensureVerificationRequestInbox } from '../../client/verificationRequestInbox';
import { isNativeMatrixSession } from '../features/verification/nativeVerification';
import { NativeVerificationInboxRenderer } from '../features/verification/NativeDeviceVerification';

const DialogHeaderStyles: CSSProperties = {
  padding: `0 ${config.space.S200} 0 ${config.space.S400}`,
  borderBottomWidth: config.borderWidth.B300,
};

type WaitingMessageProps = {
  message: string;
};
function WaitingMessage({ message }: WaitingMessageProps) {
  return (
    <Box alignItems="Center" gap="200">
      <Spinner variant="Secondary" size="200" />
      <Text size="T300">{message}</Text>
    </Box>
  );
}

type VerificationUnexpectedProps = { message: string; onClose: () => void };
function VerificationUnexpected({ message, onClose }: VerificationUnexpectedProps) {
  return (
    <Box direction="Column" gap="400">
      <Text>{message}</Text>
      <Button variant="Secondary" fill="Soft" onClick={onClose}>
        <Text size="B400">Close</Text>
      </Button>
    </Box>
  );
}

function VerificationWaitAccept() {
  return (
    <Box direction="Column" gap="400">
      <Text>Please accept the request from other device.</Text>
      <WaitingMessage message="Waiting for request to be accepted..." />
    </Box>
  );
}

type VerificationAcceptProps = {
  onAccept: () => Promise<void>;
};
function VerificationAccept({ onAccept }: VerificationAcceptProps) {
  const [acceptState, accept] = useAsyncCallback(onAccept);

  const accepting = acceptState.status === AsyncStatus.Loading;
  const acceptError =
    acceptState.status === AsyncStatus.Error
      ? verificationErrorMessage(acceptState.error)
      : undefined;
  return (
    <Box direction="Column" gap="400">
      <Text>Click accept to start the verification process.</Text>
      <Button
        variant="Primary"
        fill="Solid"
        onClick={accept}
        before={accepting && <Spinner size="100" variant="Primary" fill="Solid" />}
        disabled={accepting}
      >
        <Text size="B400">Accept</Text>
      </Button>
      {acceptError && <Text size="T200">{acceptError}</Text>}
    </Box>
  );
}

function VerificationWaitStart() {
  return (
    <Box direction="Column" gap="400">
      <Text>Verification request has been accepted.</Text>
      <WaitingMessage message="Waiting for the response from other device..." />
    </Box>
  );
}

type VerificationStartProps = {
  onStart: () => Promise<void>;
};
function AutoVerificationStart({ onStart }: VerificationStartProps) {
  const [startError, setStartError] = useState<string>();
  const start = useCallback(async () => {
    setStartError(undefined);
    try {
      await onStart();
    } catch (error) {
      setStartError(verificationErrorMessage(error));
    }
  }, [onStart]);

  useEffect(() => {
    void start();
  }, [start]);

  return (
    <Box direction="Column" gap="400">
      {!startError && <WaitingMessage message="Starting verification using emoji comparison..." />}
      {startError && (
        <>
          <Text size="T200">{startError}</Text>
          <Button variant="Secondary" fill="Soft" onClick={() => void start()}>
            <Text size="B400">Retry</Text>
          </Button>
        </>
      )}
    </Box>
  );
}

function CompareEmoji({ sasData }: { sasData: ShowSasCallbacks }) {
  const [confirmState, confirm] = useAsyncCallback(useCallback(() => sasData.confirm(), [sasData]));

  const confirming = confirmState.status === AsyncStatus.Loading;
  const confirmed = confirmState.status === AsyncStatus.Success;
  const confirmError =
    confirmState.status === AsyncStatus.Error
      ? verificationErrorMessage(confirmState.error)
      : undefined;

  return (
    <Box direction="Column" gap="400">
      <Text>Confirm the emoji below are displayed on both devices, in the same order:</Text>
      <Box
        className={ContainerColor({ variant: 'SurfaceVariant' })}
        style={{
          borderRadius: config.radii.R400,
          padding: config.space.S500,
        }}
        gap="700"
        wrap="Wrap"
        justifyContent="Center"
      >
        {sasData.sas.emoji?.map(([emoji, name], index) => (
          <Box
            // eslint-disable-next-line react/no-array-index-key
            key={`${emoji}${name}${index}`}
            direction="Column"
            gap="100"
            justifyContent="Center"
            alignItems="Center"
          >
            <Text size="H1">{emoji}</Text>
            <Text size="T200">{name}</Text>
          </Box>
        ))}
      </Box>
      {confirmed && <WaitingMessage message="Waiting for the other device to finish..." />}
      <Box direction="Column" gap="200">
        <Button
          type="button"
          variant="Primary"
          fill="Soft"
          onClick={confirm}
          disabled={confirming || confirmed}
          before={confirming && <Spinner size="100" variant="Primary" />}
        >
          <Text size="B400">They Match</Text>
        </Button>
        <Button
          type="button"
          variant="Critical"
          fill="Soft"
          onClick={() => sasData.mismatch()}
          disabled={confirming || confirmed}
        >
          <Text size="B400">Do not Match</Text>
        </Button>
      </Box>
      {confirmError && <Text size="T200">{confirmError}</Text>}
    </Box>
  );
}

type SasVerificationProps = {
  verifier: Verifier;
  onVerifierCancel: () => void;
};
function SasVerification({ verifier, onVerifierCancel }: SasVerificationProps) {
  const [sasData, setSasData] = useState<ShowSasCallbacks | undefined>(() =>
    getInitialSasCallbacks(verifier),
  );
  const [verifyError, setVerifyError] = useState<string>();
  const [verifyAttempt, setVerifyAttempt] = useState(0);

  useVerifierShowSas(verifier, setSasData);
  useVerifierCancel(verifier, onVerifierCancel);

  useEffect(() => {
    let disposed = false;
    setVerifyError(undefined);
    verifier.verify().catch((error) => {
      if (disposed) return;
      if (verifier.hasBeenCancelled) {
        onVerifierCancel();
        return;
      }
      setVerifyError(verificationErrorMessage(error));
    });

    return () => {
      disposed = true;
    };
  }, [verifier, onVerifierCancel, verifyAttempt]);

  if (sasData) {
    return <CompareEmoji sasData={sasData} />;
  }

  return (
    <Box direction="Column" gap="400">
      {!verifyError && <WaitingMessage message="Starting verification using emoji comparison..." />}
      {verifyError && (
        <>
          <Text size="T200">{verifyError}</Text>
          <Button
            variant="Secondary"
            fill="Soft"
            onClick={() => setVerifyAttempt((attempt) => attempt + 1)}
          >
            <Text size="B400">Retry</Text>
          </Button>
        </>
      )}
    </Box>
  );
}

type VerificationDoneProps = {
  onExit: () => void;
};
function VerificationDone({ onExit }: VerificationDoneProps) {
  return (
    <Box direction="Column" gap="400">
      <div>
        <Text>Your device is verified.</Text>
      </div>
      <Button variant="Primary" fill="Solid" onClick={onExit}>
        <Text size="B400">Okay</Text>
      </Button>
    </Box>
  );
}

type VerificationCanceledProps = {
  onClose: () => void;
};
function VerificationCanceled({ onClose }: VerificationCanceledProps) {
  return (
    <Box direction="Column" gap="400">
      <Text>Verification has been canceled.</Text>
      <Button variant="Secondary" fill="Soft" onClick={onClose}>
        <Text size="B400">Close</Text>
      </Button>
    </Box>
  );
}

type DeviceVerificationProps = {
  request: VerificationRequest;
  onExit: () => void;
};
export function DeviceVerification({ request, onExit }: DeviceVerificationProps) {
  const requestPhase = useVerificationRequestPhase(request);
  const [verifierCancelled, setVerifierCancelled] = useState(false);
  const [cancelError, setCancelError] = useState<string>();
  const phase = phaseFromVerifierCancellation(requestPhase, verifierCancelled);

  useEffect(() => {
    setVerifierCancelled(false);
  }, [request]);

  const handleCancel = useCallback(async () => {
    setCancelError(undefined);
    try {
      await cancelVerificationRequestForExit(request);
    } catch (error) {
      setCancelError(verificationErrorMessage(error));
      return;
    }
    onExit();
  }, [request, onExit]);

  const handleVerifierCancel = useCallback(() => {
    setVerifierCancelled(true);
  }, []);

  const handleAccept = useCallback(() => request.accept(), [request]);
  const handleStart = useCallback(async () => {
    await request.startVerification(VerificationMethod.Sas);
  }, [request]);

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
          <Dialog variant="Surface">
            <Header style={DialogHeaderStyles} variant="Surface" size="500">
              <Box grow="Yes">
                <Text size="H4">Device Verification</Text>
              </Box>
              <IconButton size="300" radii="300" onClick={() => void handleCancel()}>
                <Icon src={Icons.Cross} />
              </IconButton>
            </Header>
            <Box style={{ padding: config.space.S400 }} direction="Column" gap="400">
              {phase === VerificationPhase.Requested &&
                (request.initiatedByMe ? (
                  <VerificationWaitAccept />
                ) : (
                  <VerificationAccept onAccept={handleAccept} />
                ))}
              {phase === VerificationPhase.Ready &&
                (request.initiatedByMe ? (
                  <AutoVerificationStart onStart={handleStart} />
                ) : (
                  <VerificationWaitStart />
                ))}
              {phase === VerificationPhase.Started &&
                (request.verifier ? (
                  <SasVerification
                    verifier={request.verifier}
                    onVerifierCancel={handleVerifierCancel}
                  />
                ) : (
                  <VerificationUnexpected
                    message="Unexpected Error! Verification is started but verifier is missing."
                    onClose={handleCancel}
                  />
                ))}
              {phase === VerificationPhase.Done && <VerificationDone onExit={onExit} />}
              {phase === VerificationPhase.Cancelled && <VerificationCanceled onClose={onExit} />}
              {cancelError && <Text size="T200">Could not cancel verification: {cancelError}</Text>}
            </Box>
          </Dialog>
        </FocusTrap>
      </OverlayCenter>
    </Overlay>
  );
}

function LegacyReceiveSelfDeviceVerification() {
  const mx = useMatrixClient();
  const inbox = useMemo(() => ensureVerificationRequestInbox(mx), [mx]);
  const [requests, setRequests] = useState<VerificationRequest[]>(() => inbox.getSnapshot());

  useEffect(() => {
    const refresh = () => setRequests(inbox.getSnapshot());
    const unsubscribe = inbox.subscribe(refresh);
    const inProgress =
      mx.getCrypto()?.getVerificationRequestsToDeviceInProgress(mx.getSafeUserId()) ?? [];
    inbox.hydrate(inProgress);
    refresh();
    return unsubscribe;
  }, [inbox, mx]);

  const request = requests[0];

  const handleExit = useCallback(() => {
    if (request) inbox.dismiss(request);
  }, [inbox, request]);

  if (!request) return null;

  return (
    <DeviceVerification
      key={request.transactionId ?? `${request.otherUserId}:${request.otherDeviceId ?? ''}`}
      request={request}
      onExit={handleExit}
    />
  );
}

export function ReceiveSelfDeviceVerification() {
  if (isNativeMatrixSession()) {
    return <NativeVerificationInboxRenderer />;
  }
  return <LegacyReceiveSelfDeviceVerification />;
}
