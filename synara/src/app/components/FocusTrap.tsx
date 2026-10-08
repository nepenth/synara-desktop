import React, { cloneElement, isValidElement, useCallback, useMemo, useRef } from 'react';
import FocusTrapReact, { type FocusTrapProps } from 'focus-trap-react';

type ChildProps = { ref?: React.Ref<HTMLElement>; tabIndex?: number };

const assignRef = <T,>(ref: React.Ref<T> | undefined, value: T | null) => {
  if (typeof ref === 'function') ref(value);
  else if (ref) (ref as React.MutableRefObject<T | null>).current = value;
};

/**
 * `focus-trap-react` with a fallback focus target.
 *
 * focus-trap throws when it activates a trap that has no tabbable node, for
 * example a menu whose only item is disabled. That error reached the error
 * boundary as "This screen ran into a problem". Here the trap's own container
 * is the fallback: it gets `tabIndex={-1}` so it can take focus, and callers
 * that pass their own `fallbackFocus` keep it.
 */
export default function FocusTrap({ focusTrapOptions, children, ...props }: FocusTrapProps) {
  const containerRef = useRef<HTMLElement | null>(null);
  const child = isValidElement<ChildProps>(children) ? children : undefined;
  const childRef = child?.props.ref;

  const setContainer = useCallback(
    (node: HTMLElement | null) => {
      containerRef.current = node;
      assignRef(childRef, node);
    },
    [childRef]
  );

  const options = useMemo(
    () => ({
      fallbackFocus: () => containerRef.current ?? document.body,
      ...focusTrapOptions,
    }),
    [focusTrapOptions]
  );

  return (
    <FocusTrapReact {...props} focusTrapOptions={options}>
      {child
        ? cloneElement(child, {
            ref: setContainer,
            tabIndex: child.props.tabIndex ?? -1,
          })
        : children}
    </FocusTrapReact>
  );
}
