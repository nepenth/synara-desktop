import React, { ComponentProps } from 'react';
import { Text, as } from 'folds';
import classNames from 'classnames';
import * as css from './styles.css';

/** Row text: the nav type scale (15px/20px) comes from the style, not a Text size. */
export const NavItemContent = as<'p', ComponentProps<typeof Text>>(
  ({ className, ...props }, ref) => (
    <Text
      className={classNames(css.NavItemContent, className)}
      size="Inherit"
      {...props}
      ref={ref}
    />
  )
);
