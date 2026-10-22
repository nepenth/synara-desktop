import React, { useId } from 'react';
import { Box, Button, config, Overlay, OverlayBackdrop, OverlayCenter, Text } from 'folds';
import FocusTrap from '../../components/FocusTrap';
import { Dialog } from '../../components/modal-dialog';
import { stopPropagation } from '../../utils/keyboard';
import { REDACT_CONFIRM_COPY } from './nativeTimelineRedact';

type NativeTimelineRedactConfirmProps = {
  onRemove: () => void;
  onCancel: () => void;
};

/**
 * Asks before a message is redacted for everyone. Cancel is the default and
 * holds focus when the dialog opens; Escape and a click outside also cancel.
 * The row puts focus back on its menu button once the dialog is gone.
 */
export function NativeTimelineRedactConfirm({
  onRemove,
  onCancel,
}: NativeTimelineRedactConfirmProps) {
  const questionId = useId();

  return (
    <Overlay
      open
      backdrop={<OverlayBackdrop />}
      onContextMenu={(event: React.MouseEvent) => event.stopPropagation()}
    >
      <OverlayCenter>
        <FocusTrap
          focusTrapOptions={{
            initialFocus: false,
            setReturnFocus: false,
            onDeactivate: onCancel,
            clickOutsideDeactivates: true,
            escapeDeactivates: stopPropagation,
          }}
        >
          <Dialog
            variant="Surface"
            role="alertdialog"
            aria-labelledby={questionId}
            data-native-timeline-redact-confirm="true"
          >
            <Box style={{ padding: config.space.S400 }} direction="Column" gap="400">
              <Text id={questionId} priority="400">
                {REDACT_CONFIRM_COPY.question}
              </Text>
              <Box gap="200">
                <Button variant="Critical" onClick={onRemove}>
                  <Text size="B400">{REDACT_CONFIRM_COPY.remove}</Text>
                </Button>
                <Button variant="Secondary" fill="Soft" onClick={onCancel} autoFocus>
                  <Text size="B400">{REDACT_CONFIRM_COPY.cancel}</Text>
                </Button>
              </Box>
            </Box>
          </Dialog>
        </FocusTrap>
      </OverlayCenter>
    </Overlay>
  );
}
