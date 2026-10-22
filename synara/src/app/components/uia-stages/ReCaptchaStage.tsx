import React, { useState } from 'react';
import { Dialog, Text, Box, Button, config } from 'folds';
import { openExternalUrl } from '../../utils/appLinks';
import { StageComponentProps } from './types';
import { recaptchaFallbackUrl } from './recaptchaFallback';

function ReCaptchaErrorDialog({
  title,
  message,
  onCancel,
}: {
  title: string;
  message: string;
  onCancel: () => void;
}) {
  return (
    <Dialog>
      <Box style={{ padding: config.space.S400 }} direction="Column" gap="400">
        <Box direction="Column" gap="100">
          <Text size="H4">{title}</Text>
          <Text>{message}</Text>
        </Box>
        <Button variant="Critical" fill="None" outlined onClick={onCancel}>
          <Text as="span" size="B400">
            Cancel
          </Text>
        </Button>
      </Box>
    </Dialog>
  );
}

type ReCaptchaStageProps = StageComponentProps & {
  homeserverBaseUrl: string;
};

/**
 * The CAPTCHA is completed in the browser on the homeserver's fallback page;
 * Continue then retries the stage with only the session, which the server
 * accepts once the page reported success.
 */
export function ReCaptchaStageDialog({
  stageData,
  submitAuthDict,
  onCancel,
  homeserverBaseUrl,
}: ReCaptchaStageProps) {
  const { session } = stageData;
  const [opened, setOpened] = useState(false);
  const [openFailed, setOpenFailed] = useState(false);
  const fallbackUrl = session ? recaptchaFallbackUrl(homeserverBaseUrl, session) : undefined;

  if (!fallbackUrl || !session) {
    return (
      <ReCaptchaErrorDialog
        title="Invalid Data"
        message="This server asks for a CAPTCHA, but Synara could not find the page to show it."
        onCancel={onCancel}
      />
    );
  }

  const handleOpen = () => {
    void openExternalUrl(fallbackUrl).then((ok) => {
      setOpened(ok);
      setOpenFailed(!ok);
    });
  };

  return (
    <Dialog>
      <Box style={{ padding: config.space.S400 }} direction="Column" gap="400">
        <Box direction="Column" gap="100">
          <Text size="H4">Confirm you are not a robot</Text>
          <Text>
            This server asks for a CAPTCHA. Open it in your browser, complete it there, then come
            back and choose Continue.
          </Text>
          {openFailed && (
            <Text role="alert" size="T300">
              Synara could not open your browser. Visit {fallbackUrl} and then choose Continue.
            </Text>
          )}
        </Box>
        <Button variant={opened ? 'Secondary' : 'Primary'} onClick={handleOpen}>
          <Text as="span" size="B400">
            Open CAPTCHA in Browser
          </Text>
        </Button>
        <Button
          variant={opened ? 'Primary' : 'Secondary'}
          onClick={() => submitAuthDict({ type: 'session_only', session })}
        >
          <Text as="span" size="B400">
            Continue
          </Text>
        </Button>
        <Button variant="Critical" fill="None" outlined onClick={onCancel}>
          <Text as="span" size="B400">
            Cancel
          </Text>
        </Button>
      </Box>
    </Dialog>
  );
}
