import React, { MouseEventHandler, useCallback, useMemo, useState } from 'react';
import {
  config,
  Box,
  MenuItem,
  Text,
  Icon,
  Icons,
  IconSrc,
  RectCords,
  PopOut,
  Menu,
  Button,
  Spinner,
} from 'folds';
import FocusTrap from 'focus-trap-react';
import { stopPropagation } from '../utils/keyboard';
import { getRoomIconSrc } from '../utils/room';
import {
  normalizeRoomJoinRulePresentation,
  type RoomJoinRulePresentation,
} from '../features/matrix-dto/roomJoinRule';

export type ExtraJoinRules = Extract<RoomJoinRulePresentation, 'knock_restricted'>;
export type ExtendedJoinRules = RoomJoinRulePresentation;

type JoinRuleIcons = Record<RoomJoinRulePresentation, IconSrc>;

export const useJoinRuleIcons = (roomType?: string): JoinRuleIcons =>
  useMemo(
    () => ({
      invite: getRoomIconSrc(Icons, roomType, normalizeRoomJoinRulePresentation('invite')),
      knock: getRoomIconSrc(Icons, roomType, normalizeRoomJoinRulePresentation('knock')),
      knock_restricted: getRoomIconSrc(
        Icons,
        roomType,
        normalizeRoomJoinRulePresentation('knock_restricted')
      ),
      restricted: getRoomIconSrc(Icons, roomType, normalizeRoomJoinRulePresentation('restricted')),
      public: getRoomIconSrc(Icons, roomType, normalizeRoomJoinRulePresentation('public')),
      private: getRoomIconSrc(Icons, roomType, normalizeRoomJoinRulePresentation('private')),
    }),
    [roomType]
  );

type JoinRuleLabels = Record<RoomJoinRulePresentation, string>;
export const useRoomJoinRuleLabel = (): JoinRuleLabels =>
  useMemo(
    () => ({
      invite: 'Invite Only',
      knock: 'Knock & Invite',
      knock_restricted: 'Space Members or Knock',
      restricted: 'Space Members',
      public: 'Public',
      private: 'Invite Only',
    }),
    []
  );

type JoinRulesSwitcherProps<T extends ExtendedJoinRules[]> = {
  icons: JoinRuleIcons;
  labels: JoinRuleLabels;
  rules: T;
  value: T[number];
  onChange: (value: T[number]) => void;
  disabled?: boolean;
  changing?: boolean;
};
export function JoinRulesSwitcher<T extends ExtendedJoinRules[]>({
  icons,
  labels,
  rules,
  value,
  onChange,
  disabled,
  changing,
}: JoinRulesSwitcherProps<T>) {
  const [cords, setCords] = useState<RectCords>();

  const handleOpenMenu: MouseEventHandler<HTMLButtonElement> = (evt) => {
    setCords(evt.currentTarget.getBoundingClientRect());
  };

  const handleChange = useCallback(
    (selectedRule: unknown) => {
      const normalizedRule = normalizeRoomJoinRulePresentation(selectedRule);
      if (!normalizedRule) return;
      setCords(undefined);
      onChange(normalizedRule);
    },
    [onChange]
  );

  const normalizedValue = normalizeRoomJoinRulePresentation(value);

  return (
    <PopOut
      anchor={cords}
      position="Bottom"
      align="End"
      content={
        <FocusTrap
          focusTrapOptions={{
            initialFocus: false,
            onDeactivate: () => setCords(undefined),
            clickOutsideDeactivates: true,
            isKeyForward: (evt: KeyboardEvent) => evt.key === 'ArrowDown',
            isKeyBackward: (evt: KeyboardEvent) => evt.key === 'ArrowUp',
            escapeDeactivates: stopPropagation,
          }}
        >
          <Menu>
            <Box direction="Column" gap="100" style={{ padding: config.space.S100 }}>
              {rules.map((rule) => {
                const normalizedRule = normalizeRoomJoinRulePresentation(rule);
                return (
                  <MenuItem
                    key={rule}
                    size="300"
                    variant="Surface"
                    radii="300"
                    aria-pressed={normalizedValue === normalizedRule}
                    onClick={() => {
                      if (normalizedRule) handleChange(normalizedRule);
                    }}
                    before={<Icon size="100" src={icons[normalizedRule ?? 'restricted']} />}
                    disabled={disabled || normalizedRule === null}
                  >
                    <Box grow="Yes">
                      <Text size="T300">
                        {normalizedRule ? labels[normalizedRule] : 'Unsupported'}
                      </Text>
                    </Box>
                  </MenuItem>
                );
              })}
            </Box>
          </Menu>
        </FocusTrap>
      }
    >
      <Button
        size="300"
        variant="Secondary"
        fill="Soft"
        radii="300"
        outlined
        before={<Icon size="100" src={icons[normalizedValue ?? 'restricted']} />}
        after={
          changing ? (
            <Spinner size="100" variant="Secondary" fill="Soft" />
          ) : (
            <Icon size="100" src={Icons.ChevronBottom} />
          )
        }
        onClick={handleOpenMenu}
        disabled={disabled}
      >
        <Text size="B300">{normalizedValue ? labels[normalizedValue] : 'Unsupported'}</Text>
      </Button>
    </PopOut>
  );
}
