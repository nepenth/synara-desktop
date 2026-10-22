import React, { ReactNode } from 'react';
import {
  Overlay,
  OverlayBackdrop,
  Box,
  config,
  Text,
  TooltipProvider,
  Tooltip,
  Icons,
  Icon,
  Chip,
  IconButton,
} from 'folds';
import FocusTrap from './FocusTrap';
import { ModalDialogBox } from './modal-dialog';

export type UIAFlowOverlayProps = {
  currentStep: number;
  stepCount: number;
  children: ReactNode;
  onCancel: () => void;
};
export function UIAFlowOverlay({
  currentStep,
  stepCount,
  children,
  onCancel,
}: UIAFlowOverlayProps) {
  return (
    <Overlay open backdrop={<OverlayBackdrop />}>
      <FocusTrap focusTrapOptions={{ initialFocus: false, escapeDeactivates: false }}>
        {/* Escape cancels here, not through the trap: its onDeactivate also runs on unmount. */}
        <ModalDialogBox
          style={{
            height: '100%',
            flexGrow: 1,
            display: 'flex',
            flexDirection: 'column',
            gap: config.space.S400,
          }}
          onKeyDown={(evt) => {
            if (evt.key !== 'Escape') return;
            evt.stopPropagation();
            onCancel();
          }}
        >
          <Box grow="Yes" direction="Column" alignItems="Center" justifyContent="Center">
            {children}
          </Box>
          <Box
            style={{ padding: config.space.S200 }}
            shrink="No"
            justifyContent="Center"
            alignItems="Center"
            gap="200"
          >
            <Chip as="div" radii="Pill" outlined>
              <Text as="span" size="T300">{`Step ${currentStep}/${stepCount}`}</Text>
            </Chip>
            <TooltipProvider
              tooltip={
                <Tooltip variant="Critical">
                  <Text>Exit</Text>
                </Tooltip>
              }
              position="Top"
            >
              {(anchorRef) => (
                <IconButton
                  ref={anchorRef}
                  variant="Critical"
                  size="300"
                  onClick={onCancel}
                  radii="Pill"
                  outlined
                  aria-label="Exit"
                >
                  <Icon size="50" src={Icons.Cross} />
                </IconButton>
              )}
            </TooltipProvider>
          </Box>
        </ModalDialogBox>
      </FocusTrap>
    </Overlay>
  );
}
