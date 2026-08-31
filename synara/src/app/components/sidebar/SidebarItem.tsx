import classNames from 'classnames';
import { as, Avatar, Text, Tooltip, TooltipProvider, toRem } from 'folds';
import React, { ComponentProps, createContext, ReactNode, RefCallback, useContext } from 'react';
import * as depthCss from '../../styles/Depth.css';
import * as css from './Sidebar.css';

const SidebarItemActiveContext = createContext(false);

export const SidebarItem = as<'div', css.SidebarItemVariants>(
  ({ as: AsSidebarAvatarBox = 'div', className, active, ...props }, ref) => {
    const selected = Boolean(active);
    return (
      <SidebarItemActiveContext.Provider value={selected}>
        <AsSidebarAvatarBox
          className={classNames(css.SidebarItem({ active }), className)}
          {...props}
          ref={ref}
        />
      </SidebarItemActiveContext.Provider>
    );
  }
);

export const SidebarItemBadge = as<'div', css.SidebarItemBadgeVariants>(
  ({ as: AsSidebarBadgeBox = 'div', className, hasCount, ...props }, ref) => (
    <AsSidebarBadgeBox
      className={classNames(css.SidebarItemBadge({ hasCount }), className)}
      {...props}
      ref={ref}
    />
  )
);

export function SidebarItemTooltip({
  tooltip,
  children,
}: {
  tooltip?: ReactNode | string;
  children: (triggerRef: RefCallback<HTMLElement | SVGElement>) => ReactNode;
}) {
  if (!tooltip) {
    return children(() => undefined);
  }

  return (
    <TooltipProvider
      delay={400}
      position="Right"
      tooltip={
        <Tooltip style={{ maxWidth: toRem(280) }}>
          <Text size="H5">{tooltip}</Text>
        </Tooltip>
      }
    >
      {children}
    </TooltipProvider>
  );
}

export const SidebarAvatar = as<'div', css.SidebarAvatarVariants & ComponentProps<typeof Avatar>>(
  ({ className, size, outlined, radii, ...props }, ref) => {
    const active = useContext(SidebarItemActiveContext);
    const interactive = (props as { as?: React.ElementType }).as === 'button';
    return (
      <Avatar
        className={classNames(
          css.SidebarAvatar({ size, outlined }),
          interactive && depthCss.quietInteractiveSurface,
          className
        )}
        radii={radii}
        aria-current={interactive && active ? 'page' : undefined}
        {...props}
        ref={ref}
      />
    );
  }
);

export const SidebarFolder = as<'div', css.SidebarFolderVariants>(
  ({ as: AsSidebarFolder = 'div', className, state, ...props }, ref) => (
    <AsSidebarFolder
      className={classNames(css.SidebarFolder({ state }), className)}
      {...props}
      ref={ref}
    />
  )
);

export const SidebarFolderDropTarget = as<'div', css.SidebarFolderDropTargetVariants>(
  ({ as: AsSidebarFolderDropTarget = 'div', className, position, ...props }, ref) => (
    <AsSidebarFolderDropTarget
      className={classNames(css.SidebarFolderDropTarget({ position }), className)}
      {...props}
      ref={ref}
    />
  )
);
