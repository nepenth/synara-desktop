import React, { useMemo } from 'react';
import {
  Box,
  Header,
  Icon,
  IconButton,
  Icons,
  Overlay,
  OverlayBackdrop,
  OverlayCenter,
  Scroll,
  Text,
  config,
  toRem,
} from 'folds';
import FocusTrap from '../../components/FocusTrap';
import { Modal } from '../../components/modal-dialog';
import { stopPropagation } from '../../utils/keyboard';
import { appShortcutHelp, type AppShortcutPlatform } from './appShortcuts';

const kbdStyle: React.CSSProperties = {
  flexShrink: 0,
  fontFamily: 'inherit',
  fontSize: toRem(12),
  lineHeight: toRem(18),
  padding: `0 ${config.space.S100}`,
  borderRadius: config.radii.R300,
  border: `${config.borderWidth.B300} solid var(--synara-content-tertiary)`,
  color: 'var(--synara-content-secondary)',
  whiteSpace: 'nowrap',
};

/** Every keyboard shortcut on this platform, from the registry, by group. */
export function ShortcutHelpSheet({
  platform,
  onClose,
}: {
  platform: AppShortcutPlatform;
  onClose: () => void;
}) {
  const groups = useMemo(() => appShortcutHelp(platform), [platform]);
  return (
    <Overlay open backdrop={<OverlayBackdrop />}>
      <OverlayCenter>
        <FocusTrap
          focusTrapOptions={{
            initialFocus: false,
            onDeactivate: onClose,
            clickOutsideDeactivates: true,
            escapeDeactivates: stopPropagation,
          }}
        >
          <Modal
            variant="Surface"
            size="300"
            data-shortcut-help="true"
            // As tall as its list, never taller than a short window.
            style={{ height: 'auto', maxHeight: `calc(100vh - ${toRem(32)})` }}
          >
            <Box direction="Column" style={{ maxHeight: '85vh' }}>
              <Header
                style={{ padding: `0 ${config.space.S200} 0 ${config.space.S400}` }}
                variant="Surface"
                size="500"
              >
                <Box grow="Yes">
                  <Text as="h2" size="H4">
                    Keyboard shortcuts
                  </Text>
                </Box>
                <IconButton size="300" onClick={onClose} radii="300" aria-label="Close">
                  <Icon src={Icons.Cross} />
                </IconButton>
              </Header>
              <Scroll hideTrack visibility="Hover" size="300">
                <Box
                  direction="Column"
                  gap="500"
                  style={{ padding: config.space.S400, paddingTop: 0 }}
                >
                  {groups.map((group) => (
                    <Box as="section" key={group.scope} direction="Column" gap="200">
                      <Text as="h3" size="L400">
                        {group.label}
                      </Text>
                      <Box as="dl" direction="Column" gap="200" style={{ margin: 0 }}>
                        {group.items.map((item) => (
                          <Box
                            key={item.id}
                            alignItems="Center"
                            justifyContent="SpaceBetween"
                            gap="300"
                            data-shortcut-id={item.id}
                          >
                            <Text as="dt" size="T300">
                              {item.label}
                            </Text>
                            <dd style={{ margin: 0 }}>
                              <kbd style={kbdStyle}>{item.keys}</kbd>
                            </dd>
                          </Box>
                        ))}
                      </Box>
                    </Box>
                  ))}
                </Box>
              </Scroll>
            </Box>
          </Modal>
        </FocusTrap>
      </OverlayCenter>
    </Overlay>
  );
}
