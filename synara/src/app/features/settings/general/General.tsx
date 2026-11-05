import React, {
  ChangeEventHandler,
  FormEventHandler,
  MouseEventHandler,
  useEffect,
  useId,
  useState,
} from 'react';
import dayjs from 'dayjs';
import {
  Badge,
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
import { KeySymbol } from '../../../utils/key-symbol';
import { isMacOS } from '../../../utils/user-agent';
import { stopPropagation } from '../../../utils/keyboard';
import { useDateFormatItems } from '../../../hooks/useDateFormat';
import { SequenceCardStyle, SettingsQuietControl } from '../styles.css';
import { useClientConfig } from '../../../hooks/useClientConfig';
import { reloadApplication } from '../../../../client/initMatrix';
import { gifPickerEnabled } from '../../../utils/gifProvider';
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
import { isSynaraDesktop } from '../../../utils/desktop';
import {
  closeExperimentalWidgets,
  widgetsSettingsDescription,
} from '../../widgets/experimentalWidgets';
import { isSafeWidgetUrl } from '../../widgets/widgetUrl';
import {
  snapshotRtcTransportsNative,
  type NativeRtcTransportsSnapshot,
} from '../../matrix-rtc/nativeRtcTransports';
import { rtcCallAvailabilityCopy } from '../../matrix-rtc/liveCallChrome';
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
    </SettingsSection>
  );
}

function StorageSection() {
  return (
    <SettingsSection title="Storage">
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title="Reload application"
          description="Reset navigation and notification caches, then reload the application. Your native message store and login are kept."
          after={
            <Button
              className={SettingsQuietControl}
              onClick={() => reloadApplication()}
              variant="Critical"
              fill="None"
              size="300"
              radii="300"
              outlined
            >
              <Text size="B300">Reload</Text>
            </Button>
          }
        />
      </SequenceCard>
    </SettingsSection>
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

function WidgetsSection() {
  const [experimentalWidgetsEnabled, setExperimentalWidgetsEnabled] = useSetting(
    settingsAtom,
    'experimentalWidgetsEnabled'
  );
  const [agentWidgetEntries, setAgentWidgetEntries] = useSetting(
    settingsAtom,
    'agentWidgetEntries'
  );
  const [agentName, setAgentName] = useState('');
  const [agentUrl, setAgentUrl] = useState('');
  const agentNameId = useId();
  const agentUrlId = useId();

  if (!isSynaraDesktop()) return null;

  return (
    <SettingsSection
      title="Widgets"
      after={
        <Badge variant="Secondary" fill="Soft" radii="Pill" outlined>
          <Text as="span" size="L400">
            Beta
          </Text>
        </Badge>
      }
    >
      <SequenceCard
        className={SequenceCardStyle}
        variant="SurfaceVariant"
        direction="Column"
        gap="400"
      >
        <SettingTile
          title="Experimental widgets"
          description={widgetsSettingsDescription}
          after={
            <Switch
              variant="Primary"
              value={experimentalWidgetsEnabled}
              onChange={(value) => {
                if (!value) {
                  void closeExperimentalWidgets();
                }
                setExperimentalWidgetsEnabled(value);
              }}
            />
          }
        />
        {experimentalWidgetsEnabled && (
          <Box direction="Column" gap="200">
            <Text size="T200" priority="300">
              Agent widget URLs stay on this device. Loopback is allowed only here; room-state
              widgets cannot load localhost.
            </Text>
            {agentWidgetEntries.map((entry) => (
              <Box key={entry.id} justifyContent="SpaceBetween" alignItems="Center" gap="200">
                <Box direction="Column" grow="Yes">
                  <Text size="T300">{entry.name}</Text>
                  <Text size="T200" priority="300">
                    {entry.url}
                  </Text>
                </Box>
                <Button
                  className={SettingsQuietControl}
                  size="300"
                  variant="Critical"
                  fill="Soft"
                  onClick={() =>
                    setAgentWidgetEntries(agentWidgetEntries.filter((item) => item.id !== entry.id))
                  }
                >
                  <Text size="B300">Remove</Text>
                </Button>
              </Box>
            ))}
            <Box direction="Column" gap="200">
              <Box direction="Column" gap="100">
                <Text as="label" size="L400" priority="300" htmlFor={agentNameId}>
                  Agent name
                </Text>
                <Input
                  id={agentNameId}
                  variant="Background"
                  size="400"
                  value={agentName}
                  onChange={(evt) => setAgentName(evt.currentTarget.value)}
                />
              </Box>
              <Box direction="Column" gap="100">
                <Text as="label" size="L400" priority="300" htmlFor={agentUrlId}>
                  Widget URL
                </Text>
                <Input
                  id={agentUrlId}
                  variant="Background"
                  size="400"
                  placeholder="https://… or http://127.0.0.1:…"
                  value={agentUrl}
                  onChange={(evt) => setAgentUrl(evt.currentTarget.value)}
                />
              </Box>
              <Button
                className={SettingsQuietControl}
                size="300"
                variant="Secondary"
                fill="Soft"
                disabled={!agentName.trim() || !isSafeWidgetUrl(agentUrl.trim(), true)}
                onClick={() => {
                  const name = agentName.trim();
                  const url = agentUrl.trim();
                  if (!name || !url || !isSafeWidgetUrl(url, true)) return;
                  setAgentWidgetEntries([
                    ...agentWidgetEntries,
                    { id: `agent-${Date.now()}`, name, url },
                  ]);
                  setAgentName('');
                  setAgentUrl('');
                }}
              >
                <Text size="B300">Add agent widget</Text>
              </Button>
            </Box>
          </Box>
        )}
      </SequenceCard>
    </SettingsSection>
  );
}

function CallsSection() {
  const [snapshot, setSnapshot] = useState<NativeRtcTransportsSnapshot | null>(null);

  useEffect(() => {
    let cancelled = false;
    snapshotRtcTransportsNative()
      .then((next) => {
        if (!cancelled) setSnapshot(next);
      })
      .catch(() => {
        if (!cancelled) setSnapshot(null);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  if (!isSynaraDesktop()) return null;

  return (
    <SettingsSection title="Calls">
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title="Voice and video calls"
          description={rtcCallAvailabilityCopy({
            status: snapshot?.status ?? null,
            transports: snapshot?.transports,
          })}
        />
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

function Composer() {
  const [enterForNewline, setEnterForNewline] = useSetting(settingsAtom, 'enterForNewline');
  const [isMarkdown, setIsMarkdown] = useSetting(settingsAtom, 'isMarkdown');
  const [hideActivity, setHideActivity] = useSetting(settingsAtom, 'hideActivity');

  return (
    <SettingsSection title="Composer">
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title="Enter adds a new line"
          description={`Enter starts a new line and ${
            isMacOS() ? KeySymbol.Command : 'Ctrl'
          }+Enter sends the message.`}
          after={<Switch variant="Primary" value={enterForNewline} onChange={setEnterForNewline} />}
        />
      </SequenceCard>
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title="Markdown formatting"
          after={<Switch variant="Primary" value={isMarkdown} onChange={setIsMarkdown} />}
        />
      </SequenceCard>
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title="Hide typing & read receipts"
          description="Don't send typing status, and hide who is typing and who has read a room. Reading still marks rooms as read privately, so badges clear and your other devices stay in sync."
          after={<Switch variant="Primary" value={hideActivity} onChange={setHideActivity} />}
        />
      </SequenceCard>
    </SettingsSection>
  );
}

function Messages() {
  const { t } = useTranslation();
  const clientConfig = useClientConfig();
  const [hideMembershipEvents, setHideMembershipEvents] = useSetting(
    settingsAtom,
    'hideMembershipEvents'
  );
  const [hideNickAvatarEvents, setHideNickAvatarEvents] = useSetting(
    settingsAtom,
    'hideNickAvatarEvents'
  );
  const [mediaAutoLoad, setMediaAutoLoad] = useSetting(settingsAtom, 'mediaAutoLoad');
  const [gifSearchEnabled, setGifSearchEnabled] = useSetting(settingsAtom, 'gifSearchEnabled');
  const [inlineGifLinks, setInlineGifLinks] = useSetting(settingsAtom, 'inlineGifLinks');
  const [xEmbeds, setXEmbeds] = useSetting(settingsAtom, 'xEmbeds');
  const [linkPreviews, setLinkPreviews] = useSetting(settingsAtom, 'linkPreviews');
  const [syncDrafts, setSyncDrafts] = useSetting(settingsAtom, 'syncDrafts');
  const [shortenLinks, setShortenLinks] = useSetting(settingsAtom, 'shortenLinks');
  const [indexedMessageSearch, setIndexedMessageSearch] = useSetting(
    settingsAtom,
    'indexedMessageSearch'
  );
  const [showHiddenEvents, setShowHiddenEvents] = useSetting(settingsAtom, 'showHiddenEvents');
  const gifProviderAvailable = gifPickerEnabled(clientConfig.gifPicker);

  return (
    <SettingsSection title="Messages">
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title="Hide membership changes"
          description="Do not show join, leave, and invite events in the timeline."
          after={
            <Switch
              variant="Primary"
              value={hideMembershipEvents}
              onChange={setHideMembershipEvents}
            />
          }
        />
      </SequenceCard>
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title="Hide profile changes"
          description="Do not show display name and avatar changes in the timeline."
          after={
            <Switch
              variant="Primary"
              value={hideNickAvatarEvents}
              onChange={setHideNickAvatarEvents}
            />
          }
        />
      </SequenceCard>
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title="Load media automatically"
          description="Download and show images and video previews as messages arrive. When off, media loads on request."
          after={<Switch variant="Primary" value={mediaAutoLoad} onChange={setMediaAutoLoad} />}
        />
      </SequenceCard>
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title={t('modernization.settings.gif_search.title', 'GIF search')}
          description={
            gifProviderAvailable
              ? t(
                  'modernization.settings.gif_search.description',
                  'Show GIF search in the message composer.'
                )
              : t(
                  'modernization.settings.gif_search.unavailable',
                  'GIF search is not configured by this client.'
                )
          }
          after={
            <Switch
              variant="Primary"
              value={gifProviderAvailable && gifSearchEnabled}
              disabled={!gifProviderAvailable}
              onChange={setGifSearchEnabled}
            />
          }
        />
      </SequenceCard>
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title={t('modernization.settings.inline_gif_links.title', 'Play GIF links inline')}
          description={t(
            'modernization.settings.inline_gif_links.description',
            'Show Klipy and GIPHY links as GIFs. Your device loads them from the provider, which can see your IP address.'
          )}
          after={<Switch variant="Primary" value={inlineGifLinks} onChange={setInlineGifLinks} />}
        />
      </SequenceCard>
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title={t('modernization.settings.x_embeds.title', 'Show X posts')}
          description={t(
            'modernization.settings.x_embeds.description',
            'Show the post and play its video for x.com links. Your device contacts X, which can see your IP address.'
          )}
          after={<Switch variant="Primary" value={xEmbeds} onChange={setXEmbeds} />}
        />
      </SequenceCard>
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title={t('modernization.settings.link_previews.title', 'Link previews')}
          description={t(
            'modernization.settings.link_previews.description',
            'When a message has a link, Synara fetches the page from this device to show a preview. The site can see your IP address.'
          )}
          after={<Switch variant="Primary" value={linkPreviews} onChange={setLinkPreviews} />}
        />
      </SequenceCard>
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title={t('modernization.settings.sync_drafts.title', 'Sync drafts across devices')}
          description={t(
            'modernization.settings.sync_drafts.description',
            'Unsent messages follow you to your other Synara devices, end-to-end encrypted. Devices need your recovery key or another verified device to read them.'
          )}
          after={<Switch variant="Primary" value={syncDrafts} onChange={setSyncDrafts} />}
        />
      </SequenceCard>
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title={t('modernization.settings.shorten_links.title', 'Shorten long links')}
          description={t(
            'modernization.settings.shorten_links.description',
            'Show a long web address as its site and path. The link still opens the full address, shown when you point at it.'
          )}
          after={<Switch variant="Primary" value={shortenLinks} onChange={setShortenLinks} />}
        />
      </SequenceCard>
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title={t('modernization.settings.indexed_message_search.title', 'Indexed message search')}
          description={t(
            'modernization.settings.indexed_message_search.description',
            'Local experimental index of messages this device has already seen, including encrypted rooms. Not complete homeserver history. Changing this requires a session reload.'
          )}
          after={
            <Switch
              variant="Primary"
              value={indexedMessageSearch}
              onChange={setIndexedMessageSearch}
            />
          }
        />
      </SequenceCard>
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title="Show hidden events"
          description="Show state and unsupported events that are normally hidden from the timeline."
          after={
            <Switch variant="Primary" value={showHiddenEvents} onChange={setShowHiddenEvents} />
          }
        />
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
      <Composer />
      <Messages />
      <DesktopShortcutsSection />
      <StorageSection />
      <CallsSection />
      <WidgetsSection />
    </SettingsPage>
  );
}
