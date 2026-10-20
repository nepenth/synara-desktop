import React, { ReactNode } from 'react';
import FocusTrap from './FocusTrap';
import { Overlay, OverlayBackdrop, OverlayCenter } from 'folds';
import { Modal } from './modal-dialog';
import { stopPropagation } from '../utils/keyboard';

type Modal500Props = {
  requestClose: () => void;
  children: ReactNode;
};
export function Modal500({ requestClose, children }: Modal500Props) {
  return (
    <Overlay open backdrop={<OverlayBackdrop />}>
      <OverlayCenter>
        <FocusTrap
          focusTrapOptions={{
            initialFocus: false,
            clickOutsideDeactivates: true,
            onDeactivate: requestClose,
            escapeDeactivates: stopPropagation,
          }}
        >
          <Modal size="500" variant="Background">
            {children}
          </Modal>
        </FocusTrap>
      </OverlayCenter>
    </Overlay>
  );
}
