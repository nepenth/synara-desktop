import React, { FormEventHandler, ReactNode, useRef } from 'react';
import {
  Box,
  Button,
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
import classNames from 'classnames';
import FocusTrap from '../FocusTrap';
import { Dialog } from '../modal-dialog';
import { InlineMessage } from '../inline-message';
import { stopPropagation } from '../../utils/keyboard';
import * as css from './ConfirmDialog.css';

export type ConfirmDialogTone = 'critical' | 'primary';

type ConfirmDialogProps = {
  /** Names the object: "Leave #design?", "Log out of Synara?". */
  title: string;
  /** One sentence stating the consequence. */
  description?: ReactNode;
  /** Extra content between the description and the footer (fields, warnings). */
  children?: ReactNode;
  confirmLabel: string;
  /** Label while `busy`; defaults to `confirmLabel`. */
  busyLabel?: string;
  /** `critical` for destructive or irreversible actions. */
  tone?: ConfirmDialogTone;
  busy?: boolean;
  confirmDisabled?: boolean;
  /** Failure of the last attempt; shown above the footer. */
  error?: string;
  cancelLabel?: string;
  /** Informational dialogs have only the confirm button. */
  hideCancel?: boolean;
  /** Optional extra action, placed at the start of the footer. */
  secondaryAction?: ReactNode;
  showClose?: boolean;
  /** Enter and the confirm button. */
  onConfirm: () => void;
  /** Cancel, the close X, Escape and clicking outside. */
  onCancel: () => void;
  className?: string;
};

/**
 * The one confirmation pattern for destructive and irreversible actions.
 *
 * `role="alertdialog"`, a sentence-case title that names the object, one
 * consequence sentence and a right-aligned footer `[Cancel] [Action]`. Enter
 * confirms, Escape and clicking outside cancel (never while `busy`), and a
 * destructive dialog takes initial focus on Cancel.
 */
export function ConfirmDialog({
  title,
  description,
  children,
  confirmLabel,
  busyLabel,
  tone = 'critical',
  busy = false,
  confirmDisabled = false,
  error,
  cancelLabel = 'Cancel',
  hideCancel = false,
  secondaryAction,
  showClose = true,
  onConfirm,
  onCancel,
  className,
}: ConfirmDialogProps) {
  const cancelRef = useRef<HTMLButtonElement>(null);
  const confirmRef = useRef<HTMLButtonElement>(null);
  const destructive = tone === 'critical';
  const variant = destructive ? 'Critical' : 'Primary';

  const handleSubmit: FormEventHandler<HTMLFormElement> = (evt) => {
    evt.preventDefault();
    if (busy || confirmDisabled) return;
    onConfirm();
  };

  const handleCancel = () => {
    if (busy) return;
    onCancel();
  };

  return (
    <Overlay open backdrop={<OverlayBackdrop />}>
      <OverlayCenter>
        <FocusTrap
          focusTrapOptions={{
            initialFocus: () =>
              (destructive && !hideCancel ? cancelRef.current : confirmRef.current) ?? false,
            onDeactivate: handleCancel,
            clickOutsideDeactivates: () => !busy,
            escapeDeactivates: (evt) => (busy ? false : stopPropagation(evt)),
          }}
        >
          <Dialog
            variant="Surface"
            role="alertdialog"
            className={classNames(css.ConfirmDialog, className)}
          >
            <Header className={css.Header} variant="Surface" size="500">
              <Box grow="Yes">
                <Text as="h2" size="H4" className={css.Title} truncate>
                  {title}
                </Text>
              </Box>
              {showClose && (
                <IconButton
                  size="300"
                  radii="300"
                  variant="Surface"
                  fill="None"
                  onClick={handleCancel}
                  disabled={busy}
                  aria-label="Close"
                >
                  <Icon src={Icons.Cross} />
                </IconButton>
              )}
            </Header>
            <form onSubmit={handleSubmit}>
              <div className={css.Body}>
                {description && <Text priority="400">{description}</Text>}
                {children}
                {error && <InlineMessage tone="critical">{error}</InlineMessage>}
              </div>
              <div className={css.Footer}>
                {secondaryAction && (
                  <>
                    {secondaryAction}
                    <span className={css.FooterSpacer} aria-hidden />
                  </>
                )}
                {!hideCancel && (
                  <Button
                    ref={cancelRef}
                    type="button"
                    variant="Secondary"
                    fill="Soft"
                    size="400"
                    radii="300"
                    onClick={handleCancel}
                    disabled={busy}
                  >
                    <Text size="B400">{cancelLabel}</Text>
                  </Button>
                )}
                <Button
                  ref={confirmRef}
                  type="submit"
                  variant={variant}
                  fill="Solid"
                  size="400"
                  radii="300"
                  disabled={busy || confirmDisabled}
                  before={busy ? <Spinner size="200" variant={variant} fill="Solid" /> : undefined}
                >
                  <Text size="B400">{busy && busyLabel ? busyLabel : confirmLabel}</Text>
                </Button>
              </div>
            </form>
          </Dialog>
        </FocusTrap>
      </OverlayCenter>
    </Overlay>
  );
}
