import React, { MutableRefObject, ReactNode, useEffect } from 'react';
import FocusTrap from '../../FocusTrap';
import { isKeyHotkey } from 'is-hotkey';
import { Header, Menu, Scroll, config } from 'folds';

import * as css from './AutocompleteMenu.css';
import { preventScrollWithArrowKey, stopPropagation } from '../../../utils/keyboard';
import { useAlive } from '../../../hooks/useAlive';

/**
 * Lets the composer pick an open autocomplete's first suggestion (as Tab
 * does) when Enter is pressed, instead of sending. The function returns
 * false when there is nothing to pick, so Enter sends as usual.
 */
export type AutocompleteSelectFirstRef = MutableRefObject<(() => boolean) | undefined>;

export const useAutocompleteSelectFirst = (
  ref: AutocompleteSelectFirstRef | undefined,
  selectFirst: () => boolean
): void => {
  useEffect(() => {
    if (!ref) return undefined;
    // eslint-disable-next-line no-param-reassign
    ref.current = selectFirst;
    return () => {
      // eslint-disable-next-line no-param-reassign
      if (ref.current === selectFirst) ref.current = undefined;
    };
  });
};

type AutocompleteMenuProps = {
  requestClose: () => void;
  headerContent: ReactNode;
  children: ReactNode;
};
export function AutocompleteMenu({ headerContent, requestClose, children }: AutocompleteMenuProps) {
  const alive = useAlive();

  const handleDeactivate = () => {
    if (alive()) {
      // The component is unmounted so we will not call for `requestClose`
      requestClose();
    }
  };

  return (
    <div className={css.AutocompleteMenuBase}>
      <div className={css.AutocompleteMenuContainer}>
        <FocusTrap
          focusTrapOptions={{
            initialFocus: false,
            onPostDeactivate: handleDeactivate,
            returnFocusOnDeactivate: false,
            clickOutsideDeactivates: true,
            allowOutsideClick: true,
            isKeyForward: (evt: KeyboardEvent) => isKeyHotkey('arrowdown', evt),
            isKeyBackward: (evt: KeyboardEvent) => isKeyHotkey('arrowup', evt),
            escapeDeactivates: stopPropagation,
          }}
        >
          <Menu className={css.AutocompleteMenu}>
            <Header className={css.AutocompleteMenuHeader} size="400">
              {headerContent}
            </Header>
            <Scroll style={{ flexGrow: 1 }} onKeyDown={preventScrollWithArrowKey}>
              <div style={{ padding: config.space.S200 }}>{children}</div>
            </Scroll>
          </Menu>
        </FocusTrap>
      </div>
    </div>
  );
}
