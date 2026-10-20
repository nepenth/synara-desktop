import React, { ComponentProps, forwardRef, useCallback, useEffect, useId, useRef } from 'react';
import { Dialog as FoldsDialog, Modal as FoldsModal } from 'folds';
import classNames from 'classnames';
import * as css from './ModalDialog.css';
import { holdAppInert } from './appInert';

type DialogSemanticsProps = {
  /** `alertdialog` for confirmations that interrupt the user. */
  role?: 'dialog' | 'alertdialog';
  'aria-label'?: string;
  'aria-labelledby'?: string;
};

const TITLE_ATTRIBUTE = 'data-dialog-title';

/**
 * The dialog's visible title: an explicit `data-dialog-title`, a heading, or
 * the first text in its folds `Header` that is not inside a button.
 */
export const findDialogTitle = (dialog: HTMLElement): HTMLElement | undefined => {
  const explicit = dialog.querySelector<HTMLElement>(`[${TITLE_ATTRIBUTE}]`);
  if (explicit) return explicit;
  const heading = dialog.querySelector<HTMLElement>('h1, h2, h3, h4, h5, h6');
  if (heading) return heading;
  const header = dialog.querySelector('header');
  if (!header) return undefined;
  return Array.from(header.querySelectorAll<HTMLElement>('p, span')).find(
    (element) => !element.closest('button') && !!element.textContent?.trim()
  );
};

const assignRef = <T,>(ref: React.ForwardedRef<T>, value: T | null) => {
  if (typeof ref === 'function') ref(value);
  else if (ref) ref.current = value;
};

/**
 * Gives an overlay surface modal dialog semantics: `role="dialog"`,
 * `aria-modal`, a name from its title, focus on open, and the app behind it
 * made inert. Focus return stays with the surrounding FocusTrap.
 */
const useModalDialog = (
  forwardedRef: React.ForwardedRef<HTMLDivElement>,
  { 'aria-label': label, 'aria-labelledby': labelledBy }: DialogSemanticsProps
) => {
  const nodeRef = useRef<HTMLDivElement | null>(null);
  const titleId = useId();

  const setRef = useCallback(
    (node: HTMLDivElement | null) => {
      nodeRef.current = node;
      assignRef(forwardedRef, node);
    },
    [forwardedRef]
  );

  useEffect(() => {
    const dialog = nodeRef.current;
    if (!dialog) return undefined;
    const releaseInert = holdAppInert(dialog);

    if (!label && !labelledBy && !dialog.hasAttribute('aria-labelledby')) {
      const title = findDialogTitle(dialog);
      if (title) {
        if (!title.id) title.id = titleId;
        dialog.setAttribute('aria-labelledby', title.id);
      }
    }

    // FocusTrap captured the opener when it mounted (before this passive
    // effect), so moving focus here still returns it to the opener on close.
    // Leave autofocused fields alone.
    if (!dialog.contains(document.activeElement)) {
      if (!dialog.hasAttribute('tabindex')) dialog.tabIndex = -1;
      dialog.focus({ preventScroll: true });
    }

    return releaseInert;
  }, [label, labelledBy, titleId]);

  return setRef;
};

type ModalProps = ComponentProps<typeof FoldsModal> & DialogSemanticsProps;

/** folds `Modal` with modal dialog semantics. Use inside an `Overlay`. */
export const Modal = forwardRef<HTMLDivElement, ModalProps>(
  ({ role = 'dialog', className, ...props }, ref) => {
    const setRef = useModalDialog(ref, props);
    return (
      <FoldsModal
        {...props}
        className={classNames(css.ModalDialog, className)}
        role={role}
        aria-modal="true"
        ref={setRef}
      />
    );
  }
);

type DialogProps = ComponentProps<typeof FoldsDialog> & DialogSemanticsProps;

/** folds `Dialog` with modal dialog semantics. Use inside an `Overlay`. */
export const Dialog = forwardRef<HTMLDivElement, DialogProps>(
  ({ role = 'dialog', className, ...props }, ref) => {
    const setRef = useModalDialog(ref, props);
    return (
      <FoldsDialog
        {...props}
        className={classNames(css.ModalDialog, className)}
        role={role}
        aria-modal="true"
        ref={setRef}
      />
    );
  }
);

type ModalDialogBoxProps = React.HTMLAttributes<HTMLDivElement> & DialogSemanticsProps;

/** A plain `div` with modal dialog semantics, for overlays that are not a folds surface. */
export const ModalDialogBox = forwardRef<HTMLDivElement, ModalDialogBoxProps>(
  ({ role = 'dialog', className, ...props }, ref) => {
    const setRef = useModalDialog(ref, props);
    return (
      <div
        {...props}
        className={classNames(css.ModalDialog, className)}
        role={role}
        aria-modal="true"
        ref={setRef}
      />
    );
  }
);
