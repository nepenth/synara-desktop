import React, {
  ChangeEventHandler,
  FormEventHandler,
  MouseEventHandler,
  useEffect,
  useState,
} from 'react';
import dayjs from 'dayjs';
import {
  Box,
  Button,
  config,
  Header,
  Icon,
  IconButton,
  Icons,
  Input,
  Menu,
  MenuItem,
  PopOut,
  RectCords,
  Switch,
  Text,
  toRem,
} from 'folds';
import FocusTrap from '../../../components/FocusTrap';
import { useTranslation } from 'react-i18next';
import { SequenceCard } from '../../../components/sequence-card';
import { useSetting } from '../../../state/hooks/settings';
import { DateFormat, desktopPlatformSettingsAtom, settingsAtom } from '../../../state/settings';
import { SettingTile } from '../../../components/setting-tile';
import { isMacOS } from '../../../utils/user-agent';
import { stopPropagation } from '../../../utils/keyboard';
import { useDateFormatItems } from '../../../hooks/useDateFormat';
import { SequenceCardStyle, SettingsQuietControl } from '../styles.css';
import {
  getPlatformIntegrationStatus,
  buildShortcutFailureMessage,
  desktopShortcutConfigProblem,
  isDesktopPlatform,
  setPlatformShortcuts,
  supportsPlatformGlobalShortcuts,
} from '../../../platform';
import { InlineMessage } from '../../../components/inline-message';
import { UpdateSettingsTile } from '../../desktop-updater/DesktopUpdaterProvider';
import { roomListShortcut } from '../../room-list-pane/roomListPane';
import { resolveVimListKeys } from '../../desktop-keyboard/vimListKeys';
import { useTilingMode } from '../../../hooks/useTilingMode';
import { SettingsPage, SettingsSection } from '../../../components/settings-layout';

type DateHintProps = {
  hasChanges: boolean;
  handleReset: () => void;
};
function DateHint({ hasChanges, handleReset }: DateHintProps) {
  const [anchor, setAnchor] = useState<RectCords>();
  const categoryPadding = { padding: config.space.S200, paddingTop: 0 };

  const handleOpenMenu: MouseEventHandler<HTMLElement> = (evt) => {
    setAnchor(evt.currentTarget.getBoundingClientRect());
  };
  return (
    <PopOut
      anchor={anchor}
      position="Top"
      align="End"
      content={
        <FocusTrap
          focusTrapOptions={{
            initialFocus: false,
            onDeactivate: () => setAnchor(undefined),
            clickOutsideDeactivates: true,
            escapeDeactivates: stopPropagation,
          }}
        >
          <Menu style={{ maxHeight: '85vh', overflowY: 'auto' }}>
            <Header size="300" style={{ padding: `0 ${config.space.S200}` }}>
              <Text size="L400">Formatting</Text>
            </Header>

            <Box direction="Column">
              <Box style={categoryPadding} direction="Column">
                <Header size="300">
                  <Text size="L400">Year</Text>
                </Header>
                <Box direction="Column" tabIndex={0} gap="100">
                  <Text size="T300">
                    YY
                    <Text as="span" size="Inherit" priority="300">
                      {': '}
                      Two-digit year
                    </Text>{' '}
                  </Text>
                  <Text size="T300">
                    YYYY
                    <Text as="span" size="Inherit" priority="300">
                      {': '}Four-digit year
                    </Text>
                  </Text>
                </Box>
              </Box>

              <Box style={categoryPadding} direction="Column">
                <Header size="300">
                  <Text size="L400">Month</Text>
                </Header>
                <Box direction="Column" tabIndex={0} gap="100">
                  <Text size="T300">
                    M
                    <Text as="span" size="Inherit" priority="300">
                      {': '}The month
                    </Text>
                  </Text>
                  <Text size="T300">
                    MM
                    <Text as="span" size="Inherit" priority="300">
                      {': '}Two-digit month
                    </Text>{' '}
                  </Text>
                  <Text size="T300">
                    MMM
                    <Text as="span" size="Inherit" priority="300">
                      {': '}Short month name
                    </Text>
                  </Text>
                  <Text size="T300">
                    MMMM
                    <Text as="span" size="Inherit" priority="300">
                      {': '}Full month name
                    </Text>
                  </Text>
                </Box>
              </Box>

              <Box style={categoryPadding} direction="Column">
                <Header size="300">
                  <Text size="L400">Day of the month</Text>
                </Header>
                <Box direction="Column" tabIndex={0} gap="100">
                  <Text size="T300">
                    D
                    <Text as="span" size="Inherit" priority="300">
                      {': '}Day of the month
                    </Text>
                  </Text>
                  <Text size="T300">
                    DD
                    <Text as="span" size="Inherit" priority="300">
                      {': '}Two-digit day of the month
                    </Text>
                  </Text>
                </Box>
              </Box>
              <Box style={categoryPadding} direction="Column">
                <Header size="300">
                  <Text size="L400">Day of the week</Text>
                </Header>
                <Box direction="Column" tabIndex={0} gap="100">
                  <Text size="T300">
                    d
                    <Text as="span" size="Inherit" priority="300">
                      {': '}Day of the week (Sunday = 0)
                    </Text>
                  </Text>
                  <Text size="T300">
                    dd
                    <Text as="span" size="Inherit" priority="300">
                      {': '}Two-letter day name
                    </Text>
                  </Text>
                  <Text size="T300">
                    ddd
                    <Text as="span" size="Inherit" priority="300">
                      {': '}Short day name
                    </Text>
                  </Text>
                  <Text size="T300">
                    dddd
                    <Text as="span" size="Inherit" priority="300">
                      {': '}Full day name
                    </Text>
                  </Text>
                </Box>
              </Box>
            </Box>
          </Menu>
        </FocusTrap>
      }
    >
      {hasChanges ? (
        <IconButton
          className={SettingsQuietControl}
          tabIndex={-1}
          onClick={handleReset}
          type="reset"
          variant="Secondary"
          fill="None"
          size="300"
          radii="300"
          aria-label="Reset date format"
        >
          <Icon src={Icons.Cross} size="100" />
        </IconButton>
      ) : (
        <IconButton
          className={SettingsQuietControl}
          tabIndex={-1}
          onClick={handleOpenMenu}
          type="button"
          variant="Secondary"
          fill="None"
          size="300"
          radii="300"
          aria-label="Date format hints"
          aria-expanded={!!anchor}
          aria-haspopup="dialog"
        >
          <Icon style={{ opacity: config.opacity.P300 }} size="100" src={Icons.Info} />
        </IconButton>
      )}
    </PopOut>
  );
}

type CustomDateFormatProps = {
  value: string;
  onChange: (format: string) => void;
};
function CustomDateFormat({ value, onChange }: CustomDateFormatProps) {
  const [dateFormatCustom, setDateFormatCustom] = useState(value);

  useEffect(() => {
    setDateFormatCustom(value);
  }, [value]);

  const handleChange: ChangeEventHandler<HTMLInputElement> = (evt) => {
    const format = evt.currentTarget.value;
    setDateFormatCustom(format);
  };

  const handleReset = () => {
    setDateFormatCustom(value);
  };

  const handleSubmit: FormEventHandler<HTMLFormElement> = (evt) => {
    evt.preventDefault();

    const target = evt.target as HTMLFormElement | undefined;
    const customDateFormatInput = target?.customDateFormatInput as HTMLInputElement | undefined;
    const format = customDateFormatInput?.value;
    if (!format) return;

    onChange(format);
  };

  const hasChanges = dateFormatCustom !== value;
  return (
    <SettingTile>
      <Box as="form" onSubmit={handleSubmit} gap="200">
        <Box grow="Yes" direction="Column">
          <Input
            required
            name="customDateFormatInput"
            value={dateFormatCustom}
            onChange={handleChange}
            maxLength={16}
            autoComplete="off"
            variant="Secondary"
            radii="300"
            style={{ paddingRight: config.space.S200 }}
            after={<DateHint hasChanges={hasChanges} handleReset={handleReset} />}
          />
        </Box>
        <Button
          className={SettingsQuietControl}
          size="400"
          variant={hasChanges ? 'Success' : 'Secondary'}
          fill="Soft"
          outlined
          radii="300"
          disabled={!hasChanges}
          type="submit"
        >
          <Text size="B400">Save</Text>
        </Button>
      </Box>
    </SettingTile>
  );
}

type PresetDateFormatProps = {
  value: string;
  onChange: (format: string) => void;
};
function PresetDateFormat({ value, onChange }: PresetDateFormatProps) {
  const [menuCords, setMenuCords] = useState<RectCords>();
  const dateFormatItems = useDateFormatItems();

  const getDisplayDate = (format: string): string =>
    format !== '' ? dayjs().format(format) : 'Custom';

  const handleMenu: MouseEventHandler<HTMLButtonElement> = (evt) => {
    setMenuCords(evt.currentTarget.getBoundingClientRect());
  };

  const handleSelect = (format: DateFormat) => {
    onChange(format);
    setMenuCords(undefined);
  };

  return (
    <>
      <Button
        className={SettingsQuietControl}
        size="300"
        variant="Secondary"
        outlined
        fill="None"
        radii="300"
        after={<Icon size="300" src={Icons.ChevronBottom} />}
        onClick={handleMenu}
        aria-haspopup="menu"
        aria-expanded={menuCords !== undefined}
      >
        <Text size="T300">
          {getDisplayDate(dateFormatItems.find((i) => i.format === value)?.format ?? value)}
        </Text>
      </Button>
      <PopOut
        anchor={menuCords}
        offset={5}
        position="Bottom"
        align="End"
        content={
          <FocusTrap
            focusTrapOptions={{
              initialFocus: false,
              onDeactivate: () => setMenuCords(undefined),
              clickOutsideDeactivates: true,
              isKeyForward: (evt: KeyboardEvent) =>
                evt.key === 'ArrowDown' || evt.key === 'ArrowRight',
              isKeyBackward: (evt: KeyboardEvent) =>
                evt.key === 'ArrowUp' || evt.key === 'ArrowLeft',
              escapeDeactivates: stopPropagation,
            }}
          >
            <Menu>
              <Box direction="Column" gap="100" style={{ padding: config.space.S100 }}>
                {dateFormatItems.map((item) => (
                  <MenuItem
                    key={item.format}
                    className={SettingsQuietControl}
                    size="300"
                    variant="Surface"
                    radii="300"
                    aria-pressed={value === item.format}
                    after={
                      value === item.format ? <Icon size="100" src={Icons.Check} /> : undefined
                    }
                    onClick={() => handleSelect(item.format)}
                  >
                    <Text size="T300">{getDisplayDate(item.format)}</Text>
                  </MenuItem>
                ))}
              </Box>
            </Menu>
          </FocusTrap>
        }
      />
    </>
  );
}

function SelectDateFormat() {
  const [dateFormatString, setDateFormatString] = useSetting(settingsAtom, 'dateFormatString');
  const [selectedDateFormat, setSelectedDateFormat] = useState(dateFormatString);
  const customDateFormat = selectedDateFormat === '';

  const handlePresetChange = (format: string) => {
    setSelectedDateFormat(format);
    if (format !== '') {
      setDateFormatString(format);
    }
  };

  return (
    <>
      <SettingTile
        title="Date format"
        description={customDateFormat ? dayjs().format(dateFormatString) : ''}
        after={<PresetDateFormat value={selectedDateFormat} onChange={handlePresetChange} />}
      />
      {customDateFormat && (
        <CustomDateFormat value={dateFormatString} onChange={setDateFormatString} />
      )}
    </>
  );
}

function normalizeShortcut(value: string): string {
  return value.trim();
}

function DesktopShortcutsSection() {
  const { t } = useTranslation();
  const shortcutsSupported = supportsPlatformGlobalShortcuts();
  const [storedShowShortcut, setStoredShowShortcut] = useSetting(
    desktopPlatformSettingsAtom,
    'desktopShortcutShow'
  );
  const [storedLaterShortcut, setStoredLaterShortcut] = useSetting(
    desktopPlatformSettingsAtom,
    'desktopShortcutLater'
  );
  const [storedNotificationsShortcut, setStoredNotificationsShortcut] = useSetting(
    desktopPlatformSettingsAtom,
    'desktopShortcutNotifications'
  );

  const [showShortcut, setShowShortcut] = useState(storedShowShortcut);
  const [laterShortcut, setLaterShortcut] = useState(storedLaterShortcut);
  const [notificationsShortcut, setNotificationsShortcut] = useState(storedNotificationsShortcut);
  const [error, setError] = useState<string>();
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    setShowShortcut(storedShowShortcut);
    setLaterShortcut(storedLaterShortcut);
    setNotificationsShortcut(storedNotificationsShortcut);
    setError(undefined);
  }, [storedShowShortcut, storedLaterShortcut, storedNotificationsShortcut]);

  const normalizedShowShortcut = normalizeShortcut(showShortcut);
  const normalizedLaterShortcut = normalizeShortcut(laterShortcut);
  const normalizedNotificationsShortcut = normalizeShortcut(notificationsShortcut);
  // An empty field turns that shortcut off; only enabled ones must differ.
  const hasDuplicate =
    desktopShortcutConfigProblem([
      normalizedShowShortcut,
      normalizedLaterShortcut,
      normalizedNotificationsShortcut,
    ]) === 'duplicate';

  const hasChanges =
    normalizedShowShortcut !== storedShowShortcut ||
    normalizedLaterShortcut !== storedLaterShortcut ||
    normalizedNotificationsShortcut !== storedNotificationsShortcut;

  const applyError = hasDuplicate
    ? t('modernization.settings.desktop_shortcuts.duplicate', 'Shortcuts must be unique.')
    : undefined;

  const handleSave = async (evt: React.FormEvent<HTMLFormElement>) => {
    evt.preventDefault();
    if (!shortcutsSupported) return;
    if (!hasChanges || applyError) return;

    setSaving(true);
    setError(undefined);
    const result = await setPlatformShortcuts({
      show: normalizedShowShortcut,
      later: normalizedLaterShortcut,
      notifications: normalizedNotificationsShortcut,
    });

    if (!result.success) {
      const status = await getPlatformIntegrationStatus();
      setError(
        buildShortcutFailureMessage(result, status) ||
          t('modernization.settings.desktop_shortcuts.failed', 'Failed to save shortcuts.')
      );
      setSaving(false);
      return;
    }

    setStoredShowShortcut(normalizedShowShortcut);
    setStoredLaterShortcut(normalizedLaterShortcut);
    setStoredNotificationsShortcut(normalizedNotificationsShortcut);
    setSaving(false);
  };

  const fields = [
    {
      id: 'show',
      label: t('modernization.settings.desktop_shortcuts.show_label', 'Show Synara'),
      value: showShortcut,
      set: setShowShortcut,
    },
    {
      id: 'later',
      label: t('modernization.settings.desktop_shortcuts.later_label', 'Open Later'),
      value: laterShortcut,
      set: setLaterShortcut,
    },
    {
      id: 'notifications',
      label: t(
        'modernization.settings.desktop_shortcuts.notifications_label',
        'Open Notifications'
      ),
      value: notificationsShortcut,
      set: setNotificationsShortcut,
    },
  ];

  return (
    <SettingsSection
      title={t('modernization.settings.desktop_shortcuts.title', 'Keyboard shortcuts')}
      description={
        shortcutsSupported
          ? t(
              'modernization.settings.desktop_shortcuts.description',
              'Open Synara screens from anywhere. These keys stop working in other apps while Synara runs; clear a field to turn that shortcut off.'
            )
          : t(
              'modernization.settings.desktop_shortcuts.unavailable',
              'Desktop shortcuts are only available in the desktop app.'
            )
      }
    >
      <Box as="form" onSubmit={handleSave} direction="Column">
        {fields.map((field) => (
          <SequenceCard
            key={field.id}
            className={SequenceCardStyle}
            variant="SurfaceVariant"
            direction="Column"
          >
            <SettingTile
              title={field.label}
              after={
                <Input
                  size="300"
                  radii="300"
                  variant="Secondary"
                  outlined
                  style={{ width: toRem(180) }}
                  aria-label={field.label}
                  value={field.value}
                  onChange={(evt) => {
                    setError(undefined);
                    field.set(evt.currentTarget.value);
                  }}
                  disabled={!shortcutsSupported}
                  placeholder={t('modernization.settings.desktop_shortcuts.off_placeholder', 'Off')}
                />
              }
            />
          </SequenceCard>
        ))}
        <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
          <SettingTile
            after={
              <Button
                className={SettingsQuietControl}
                size="300"
                radii="300"
                variant={hasChanges ? 'Primary' : 'Secondary'}
                fill={hasChanges ? 'Soft' : 'None'}
                outlined={!hasChanges}
                disabled={!shortcutsSupported || saving || !hasChanges || !!applyError}
                type="submit"
              >
                <Text size="B300">
                  {saving
                    ? t('modernization.settings.desktop_shortcuts.saving', 'Saving…')
                    : t('modernization.settings.desktop_shortcuts.apply', 'Apply shortcuts')}
                </Text>
              </Button>
            }
          >
            {applyError || error ? (
              <InlineMessage tone="critical">{error ?? applyError}</InlineMessage>
            ) : (
              <Text size="T200" priority="300">
                {t(
                  'modernization.settings.desktop_shortcuts.hint',
                  'Example: CommandOrControl+Shift+S'
                )}
              </Text>
            )}
          </SettingTile>
        </SequenceCard>
      </Box>
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title={t(
            'modernization.settings.desktop_shortcuts.room_list_label',
            'Show or hide the room list'
          )}
          description={t(
            'modernization.settings.desktop_shortcuts.room_list_description',
            'Works in the Synara window, also while typing. On macOS it is in the View menu too.'
          )}
          after={
            <Text as="kbd" size="T300" priority="300">
              {roomListShortcut(isMacOS()).label}
            </Text>
          }
        />
        <VimListKeysTile />
      </SequenceCard>
    </SettingsSection>
  );
}

/** K4: j/k, g g, G and Enter in the room list. Not chosen yet, it follows tiling mode. */
function VimListKeysTile() {
  const [setting, setSetting] = useSetting(desktopPlatformSettingsAtom, 'vimListKeys');
  const tilingMode = useTilingMode();
  const enabled = resolveVimListKeys(setting, tilingMode);
  return (
    <SettingTile
      title="Vim-style list keys"
      description={`With a room focused in the list: j and k move down and up, g g goes to the top, G to the bottom, and Enter opens the room. Never while typing.${
        setting === 'auto' ? ' On by default in tiling mode.' : ''
      }`}
      after={
        <Switch
          variant="Primary"
          value={enabled}
          onChange={(value: boolean) => setSetting(value ? 'on' : 'off')}
          aria-label="Vim-style list keys"
        />
      }
    />
  );
}

function SoftwareUpdatesSection() {
  if (!isDesktopPlatform()) return null;

  return (
    <SettingsSection title="Updates">
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <UpdateSettingsTile />
      </SequenceCard>
    </SettingsSection>
  );
}

function DateAndTime() {
  const [hour24Clock, setHour24Clock] = useSetting(settingsAtom, 'hour24Clock');

  return (
    <SettingsSection title="Date & time">
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title="24-hour time"
          after={<Switch variant="Primary" value={hour24Clock} onChange={setHour24Clock} />}
        />
      </SequenceCard>

      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SelectDateFormat />
      </SequenceCard>
    </SettingsSection>
  );
}

type GeneralProps = {
  requestClose: () => void;
};
export function General({ requestClose }: GeneralProps) {
  return (
    <SettingsPage title="General" requestClose={requestClose}>
      <SoftwareUpdatesSection />
      <DateAndTime />
      <DesktopShortcutsSection />
    </SettingsPage>
  );
}
