import React, { useRef } from 'react';
import { Scroll } from 'folds';

import {
  Sidebar,
  SidebarContent,
  SidebarStackSeparator,
  SidebarStack,
} from '../../components/sidebar';
import {
  DirectTab,
  HomeTab,
  SpaceTabs,
  InboxTab,
  ExploreTab,
  SettingsTab,
  UnverifiedTab,
  SearchTab,
} from './sidebar';
import { CreateTab } from './sidebar/CreateTab';
import { isSynaraDesktop } from '../../utils/desktop';
import { isMacOS } from '../../utils/user-agent';

export function SidebarNav() {
  const scrollRef = useRef<HTMLDivElement>(null);
  // macOS overlay traffic lights float over the top-left corner. Reserve a
  // draggable strip above the rail so the lights never cover navigation and
  // the window stays movable. Other platforms keep native decorations.
  const overlaySpacer = isSynaraDesktop() && isMacOS();

  return (
    <Sidebar>
      {overlaySpacer && <div data-tauri-drag-region style={{ height: 28, flexShrink: 0 }} />}
      <SidebarContent
        scrollable={
          <Scroll ref={scrollRef} variant="Background" size="0">
            <SidebarStack>
              <HomeTab />
              <DirectTab />
            </SidebarStack>
            <SpaceTabs scrollRef={scrollRef} />
            <SidebarStackSeparator />
            <SidebarStack>
              <ExploreTab />
              <CreateTab />
            </SidebarStack>
          </Scroll>
        }
        sticky={
          <>
            <SidebarStackSeparator />
            <SidebarStack>
              <SearchTab />
              <UnverifiedTab />
              <InboxTab />
              <SettingsTab />
            </SidebarStack>
          </>
        }
      />
    </Sidebar>
  );
}
