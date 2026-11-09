import React, { useRef } from 'react';
import { Scroll } from 'folds';

import {
  Sidebar,
  SidebarContent,
  SidebarStackSeparator,
  SidebarStack,
} from '../../components/sidebar';
import { HomeTab, SpaceTabs, InboxTab, SettingsTab, UnverifiedTab } from './sidebar';
import { ApprovalsTab } from './sidebar/ApprovalsTab';
import { RoomListTab } from './sidebar/RoomListTab';
import { ScreenSize, useScreenSizeContext } from '../../hooks/useScreenSize';

/**
 * The rail is places, not actions (wave 3): Home, the spaces, then Activity,
 * Approvals and the account at the bottom. Create, join, new message and
 * explore live in the Home list's "+" menu; search is the list's search field
 * and ⌘K. Direct messages are a section of the Home list.
 *
 * The one control above the places shows or hides the room list (⌘⇧D). It
 * sits beside the top of the pane it folds away, outside the scroller so it
 * never scrolls off with the spaces, and stands in for the hidden list's
 * unread.
 */
export function SidebarNav() {
  const scrollRef = useRef<HTMLDivElement>(null);
  // The phone layout has no side-by-side pane to hide.
  const screenSize = useScreenSizeContext();

  return (
    <Sidebar as="nav" aria-label="Spaces">
      {screenSize !== ScreenSize.Mobile && (
        <SidebarStack>
          <RoomListTab />
        </SidebarStack>
      )}
      <SidebarContent
        scrollable={
          <Scroll ref={scrollRef} variant="Background" size="0">
            <SidebarStack>
              <HomeTab />
            </SidebarStack>
            <SpaceTabs scrollRef={scrollRef} />
          </Scroll>
        }
        sticky={
          <>
            <SidebarStackSeparator />
            <SidebarStack>
              <InboxTab />
              <ApprovalsTab />
              <UnverifiedTab />
              <SettingsTab />
            </SidebarStack>
          </>
        }
      />
    </Sidebar>
  );
}
