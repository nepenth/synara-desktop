import React, { useCallback, useEffect, useState } from 'react';
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
import { ContainerColor } from '../../styles/ContainerColor.css';
import {
  acceptNativeVerification,
  beginNativeVerificationSas,
  cancelNativeVerification,
  confirmNativeVerification,
  dismissNativeVerification,
  listNativeVerificationRequests,
  mismatchNativeVerification,
  nativeVerificationErrorMessage,
  NativeVerificationRequest,
  startNativeVerification,
  verificationRequestHasSasCodes,
  verificationRequestNeedsSasStart,
} from './nativeVerification';

const POLL_INTERVAL_MS = 500;

function Waiting({ children }: { children: string }) {
  return (
    <Box alignItems="Center" gap="200">
      <Spinner variant="Secondary" size="200" />
      <Text size="T300">{children}</Text>
    </Box>
  );
}

function NativeSas({
  request,
  update,
  fail,
}: {
  request: NativeVerificationRequest;
  update: (request: NativeVerificationRequest) => void;
  fail: () => void;
}) {
  const [submitting, setSubmitting] = useState(false);
  const act = async (
    action: (flowId: string) => Promise<NativeVerificationRequest>
  ): Promise<void> => {
    setSubmitting(true);
    try {
      update(await action(request.flowId));
    } catch {
      fail();
    } finally {
      setSubmitting(false);
    }
  };
  const emoji = request.sas?.emoji;
  const decimals = request.sas?.decimals;
  if (!verificationRequestHasSasCodes(request)) {
    return (
      <Box direction="Column" gap="200">
        <Text>
          Comparison codes are not ready. Do not confirm this session until both devices show the
          same emoji or numbers.
        </Text>
      </Box>
    );
  }

  return (
    <Box direction="Column" gap="400">
      <Text>Confirm the codes below are displayed on both devices in the same order.</Text>
      {emoji && emoji.length > 0 && (
        <Box
          className={ContainerColor({ variant: 'SurfaceVariant' })}
          style={{ borderRadius: config.radii.R400, padding: config.space.S500 }}
          gap="700"
          wrap="Wrap"
          justifyContent="Center"
        >
          {emoji.map((item, index) => (
            <Box
              key={`${item.symbol}-${item.description}-${index}`}
              direction="Column"
              gap="100"
              justifyContent="Center"
              alignItems="Center"
            >
              <Text size="H1">{item.symbol}</Text>
              <Text size="T200">{item.description}</Text>
            </Box>
          ))}
        </Box>
      )}
      {(!emoji || emoji.length === 0) && decimals && (
        <Box
          className={ContainerColor({ variant: 'SurfaceVariant' })}
          style={{ borderRadius: config.radii.R400, padding: config.space.S500 }}
          gap="500"
          justifyContent="Center"
        >
          {decimals.map((decimal) => (
            <Text key={decimal} size="H3">
              {decimal}
            </Text>
          ))}
        </Box>
      )}
      <Box direction="Column" gap="200">
        <Button
          variant="Primary"
          fill="Soft"
          disabled={submitting}
          onClick={() => void act(confirmNativeVerification)}
        >
          <Text size="B400">They Match</Text>
        </Button>
        <Button
          variant="Critical"
          fill="Soft"
          disabled={submitting}
          onClick={() => void act(mismatchNativeVerification)}
        >
          <Text size="B400">Do Not Match</Text>
        </Button>
      </Box>
    </Box>
  );
}

export function NativeDeviceVerification({
  initialRequest,
  onExit,
}: {
  initialRequest: NativeVerificationRequest;
  onExit: () => void;
}) {
  const [request, setRequest] = useState(initialRequest);
  const [error, setError] = useState(false);
  const [working, setWorking] = useState(false);
  const [sasAttempt, setSasAttempt] = useState(0);
  const shouldBeginSas = verificationRequestNeedsSasStart(request);
  const requestFlowId = request.flowId;

  useEffect(() => setRequest(initialRequest), [initialRequest]);

  useEffect(() => {
    let disposed = false;
    const poll = async () => {
      try {
        const inbox = await listNativeVerificationRequests();
        if (disposed) return;
        const refreshed = inbox.requests.find((item) => item.flowId === request.flowId);
        if (refreshed) setRequest(refreshed);
      } catch {
        if (!disposed) setError(true);
      }
    };
    const interval = window.setInterval(() => void poll(), POLL_INTERVAL_MS);
    return () => {
      disposed = true;
      window.clearInterval(interval);
    };
  }, [request.flowId]);

  useEffect(() => {
    if (!shouldBeginSas) return;
    let disposed = false;
    setWorking(true);
    beginNativeVerificationSas(requestFlowId)
      .then((next) => {
        if (!disposed) setRequest(next);
      })
      .catch(() => {
        if (!disposed) setError(true);
      })
      .finally(() => {
        if (!disposed) setWorking(false);
      });
    return () => {
      disposed = true;
    };
  }, [requestFlowId, sasAttempt, shouldBeginSas]);

  const accept = async () => {
    setWorking(true);
    try {
      setRequest(await acceptNativeVerification(request.flowId));
    } catch {
      setError(true);
    } finally {
      setWorking(false);
    }
  };

  const close = useCallback(async () => {
    setWorking(true);
    try {
      if (
        request.phase === 'done' ||
        request.phase === 'mismatched' ||
        request.phase === 'cancelled'
      ) {
        await dismissNativeVerification(request.flowId);
      } else {
        await cancelNativeVerification(request.flowId);
      }
      onExit();
    } catch {
      setError(true);
    } finally {
      setWorking(false);
    }
  }, [onExit, request]);

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
            <Header
              style={{
                padding: `0 ${config.space.S200} 0 ${config.space.S400}`,
                borderBottomWidth: config.borderWidth.B300,
              }}
              variant="Surface"
              size="500"
            >
              <Box grow="Yes">
                <Text size="H4">Device Verification</Text>
              </Box>
              <IconButton size="300" radii="300" disabled={working} onClick={() => void close()}>
                <Icon src={Icons.Cross} />
              </IconButton>
            </Header>
            <Box style={{ padding: config.space.S400 }} direction="Column" gap="400">
              {request.phase === 'requested' &&
                (request.direction === 'incoming' ? (
                  <>
                    <Text>Accept this verification request to compare security codes.</Text>
                    <Button variant="Primary" disabled={working} onClick={() => void accept()}>
                      <Text size="B400">Accept</Text>
                    </Button>
                  </>
                ) : (
                  <Waiting>Waiting for another device to accept…</Waiting>
                ))}
              {request.phase === 'ready' &&
                (request.direction === 'incoming' ? (
                  <Waiting>Waiting for the other device to start comparison…</Waiting>
                ) : (
                  <Waiting>Starting secure comparison…</Waiting>
                ))}
              {request.phase === 'started' && <Waiting>Preparing comparison codes…</Waiting>}
              {request.phase === 'sas_ready' && (
                <NativeSas request={request} update={setRequest} fail={() => setError(true)} />
              )}
              {request.phase === 'confirmed' && (
                <Waiting>Waiting for the other device to finish…</Waiting>
              )}
              {request.phase === 'done' && (
                <>
                  <Text>Your device is verified.</Text>
                  <Button variant="Primary" onClick={() => void close()}>
                    <Text size="B400">Okay</Text>
                  </Button>
                </>
              )}
              {request.phase === 'cancelled' && (
                <>
                  <Text>Verification has been canceled.</Text>
                  <Button variant="Secondary" fill="Soft" onClick={() => void close()}>
                    <Text size="B400">Close</Text>
                  </Button>
                </>
              )}
              {request.phase === 'mismatched' && (
                <>
                  <Text>The security codes did not match. Verification was canceled safely.</Text>
                  <Button variant="Secondary" fill="Soft" onClick={() => void close()}>
                    <Text size="B400">Close</Text>
                  </Button>
                </>
              )}
              {error && (
                <>
                  <Text size="T200">{nativeVerificationErrorMessage()}</Text>
                  {shouldBeginSas && (
                    <Button
                      variant="Secondary"
                      fill="Soft"
                      disabled={working}
                      onClick={() => {
                        setError(false);
                        setSasAttempt((attempt) => attempt + 1);
                      }}
                    >
                      <Text size="B400">Retry</Text>
                    </Button>
                  )}
                </>
              )}
            </Box>
          </Dialog>
        </FocusTrap>
      </OverlayCenter>
    </Overlay>
  );
}

export function NativeVerificationInboxRenderer() {
  const [request, setRequest] = useState<NativeVerificationRequest>();
  const [unavailable, setUnavailable] = useState(false);

  useEffect(() => {
    let disposed = false;
    const poll = async () => {
      try {
        const inbox = await listNativeVerificationRequests();
        if (!disposed) {
          setRequest(inbox.requests[0]);
          setUnavailable(false);
        }
      } catch {
        if (!disposed) setUnavailable(true);
      }
    };
    void poll();
    const interval = window.setInterval(() => void poll(), POLL_INTERVAL_MS);
    return () => {
      disposed = true;
      window.clearInterval(interval);
    };
  }, []);

  if (unavailable) return null;
  if (!request) return null;
  return (
    <NativeDeviceVerification
      initialRequest={request}
      onExit={() => {
        setRequest(undefined);
      }}
    />
  );
}

export function NativeStartVerification({
  deviceId,
  onExit,
}: {
  deviceId?: string;
  onExit: () => void;
}) {
  const [request, setRequest] = useState<NativeVerificationRequest>();
  const [starting, setStarting] = useState(false);
  const [error, setError] = useState(false);

  const start = async () => {
    setStarting(true);
    setError(false);
    try {
      setRequest(await startNativeVerification(deviceId));
    } catch {
      setError(true);
    } finally {
      setStarting(false);
    }
  };

  return (
    <>
      {!request && (
        <Box direction="Column" gap="200" alignItems="End">
          <Button variant="Warning" disabled={starting} onClick={() => void start()}>
            {starting && <Spinner size="100" variant="Warning" />}
            <Text size="B300">{deviceId ? 'Verify' : 'Verify from Another Device'}</Text>
          </Button>
          {error && <Text size="T200">{nativeVerificationErrorMessage()}</Text>}
        </Box>
      )}
      {request && (
        <NativeDeviceVerification
          initialRequest={request}
          onExit={() => {
            setRequest(undefined);
            onExit();
          }}
        />
      )}
    </>
  );
}
