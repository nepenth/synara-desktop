import classNames from 'classnames';
import React, { ComponentProps, forwardRef } from 'react';
import { Link } from 'react-router-dom';
import { as } from 'folds';
import * as css from './styles.css';

/**
 * A row in a navigation list. `selected` styles the current row; the link or
 * button inside it carries `aria-current="page"`. (`aria-selected` is only
 * valid on options, tabs and grid cells, not on this plain container.)
 */
export const NavItem = as<
  'div',
  {
    highlight?: boolean;
    selected?: boolean;
  } & css.RoomSelectorVariants
>(
  (
    { as: AsNavItem = 'div', className, highlight, selected, variant, radii, children, ...props },
    ref
  ) => (
    <AsNavItem
      className={classNames(css.NavItem({ variant, radii }), className)}
      data-highlight={highlight}
      data-selected={selected || undefined}
      {...props}
      ref={ref}
    >
      {children}
    </AsNavItem>
  )
);

export const NavLink = forwardRef<HTMLAnchorElement, ComponentProps<typeof Link>>(
  ({ className, ...props }, ref) => (
    <Link className={classNames(css.NavLink, className)} {...props} ref={ref} />
  )
);

export const NavButton = as<'button'>(
  ({ as: AsNavButton = 'button', className, ...props }, ref) => (
    <AsNavButton className={classNames(css.NavLink, className)} {...props} ref={ref} />
  )
);
