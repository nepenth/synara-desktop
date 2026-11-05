import React, { useState } from 'react';
import {
  Box,
  Icon,
  IconButton,
  Icons,
  Menu,
  MenuItem,
  PopOut,
  RectCords,
  Text,
  config,
  toRem,
} from 'folds';
import { useTranslation } from 'react-i18next';
import FocusTrap from '../../components/FocusTrap';
import { stopPropagation } from '../../utils/keyboard';
import * as navCss from '../../components/nav/styles.css';

export type RoomListSortOption<S extends string> = {
  value: S;
  label: string;
};

type RoomListSortMenuProps<S extends string> = {
  sectionLabel: string;
  value: S;
  options: readonly RoomListSortOption<S>[];
  onChange: (sort: S) => void;
};

/**
 * One sort menu per section, behind a 28px hover-revealed control in the
 * section header. Reused by Home, Direct messages and every space.
 */
export function RoomListSortMenu<S extends string>({
  sectionLabel,
  value,
  options,
  onChange,
}: RoomListSortMenuProps<S>) {
  const { t } = useTranslation();
  const [menuAnchor, setMenuAnchor] = useState<RectCords>();
  const activeLabel = options.find((option) => option.value === value)?.label ?? value;
  const closeMenu = () => setMenuAnchor(undefined);
  const title = t('modernization.nav.sort_section', {
    defaultValue: 'Sort {{section}}. Current: {{current}}',
    section: sectionLabel,
    current: activeLabel,
  });
  return (
    <PopOut
      anchor={menuAnchor}
      position="Bottom"
      align="End"
      content={
        <FocusTrap
          focusTrapOptions={{
            initialFocus: false,
            onDeactivate: closeMenu,
            clickOutsideDeactivates: true,
            escapeDeactivates: stopPropagation,
          }}
        >
          <Menu style={{ width: toRem(190) }}>
            <Box direction="Column" gap="100" style={{ padding: config.space.S100 }}>
              {options.map((option) => (
                <MenuItem
                  key={option.value}
                  variant="Surface"
                  fill="None"
                  size="300"
                  radii="300"
                  onClick={() => {
                    onChange(option.value);
                    closeMenu();
                  }}
                  after={value === option.value ? <Icon size="100" src={Icons.Check} /> : undefined}
                >
                  <Text size="T300">{option.label}</Text>
                </MenuItem>
              ))}
            </Box>
          </Menu>
        </FocusTrap>
      }
    >
      <IconButton
        className={navCss.NavSmallIconButton}
        size="300"
        variant="Surface"
        radii="300"
        fill={menuAnchor ? 'Soft' : 'None'}
        aria-expanded={!!menuAnchor}
        aria-label={title}
        title={title}
        onClick={(event) =>
          setMenuAnchor(menuAnchor ? undefined : event.currentTarget.getBoundingClientRect())
        }
      >
        <Icon className={navCss.NavSmallIcon} size="50" src={Icons.Sort} filled />
      </IconButton>
    </PopOut>
  );
}
