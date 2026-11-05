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

/**
 * The rail is places, not actions (wave 3): Home, the spaces, then Activity,
 * Approvals and the account at the bottom. Create, join, new message and
 * explore live in the Home list's "+" menu; search is the list's search field
 * and ⌘K. Direct messages are a section of the Home list.
 */
export function SidebarNav() {
  const scrollRef = useRef<HTMLDivElement>(null);

  return (
    <Sidebar as="nav" aria-label="Spaces">
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
