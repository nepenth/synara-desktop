import React, {
  ChangeEventHandler,
  FormEventHandler,
  KeyboardEventHandler,
  MouseEventHandler,
  useEffect,
  useState,
} from 'react';
import dayjs from 'dayjs';
import {
  as,
  Box,
  Button,
  Chip,
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
  Scroll,
  Switch,
  Text,
  toRem,
} from 'folds';
import { isKeyHotkey } from 'is-hotkey';
import FocusTrap from 'focus-trap-react';
import { useTranslation } from 'react-i18next';
import { Page, PageContent, PageHeader } from '../../../components/page';
import { SequenceCard } from '../../../components/sequence-card';
import { useSetting } from '../../../state/hooks/settings';
import {
  DateFormat,
  desktopPlatformSettingsAtom,
  MessageLayout,
  MessageSpacing,
  settingsAtom,
} from '../../../state/settings';
import { SettingTile } from '../../../components/setting-tile';
import { KeySymbol } from '../../../utils/key-symbol';
import { isMacOS } from '../../../utils/user-agent';
import {
  DarkTheme,
  LightTheme,
  Theme,
  ThemeKind,
  useSystemThemeKind,
  useTheme,
  useThemeNames,
  useThemes,
} from '../../../hooks/useTheme';
import { stopPropagation } from '../../../utils/keyboard';
import { useMessageLayoutItems } from '../../../hooks/useMessageLayout';
import { useMessageSpacingItems } from '../../../hooks/useMessageSpacing';
import { useDateFormatItems } from '../../../hooks/useDateFormat';
import { SequenceCardStyle } from '../styles.css';
import { useClientConfig } from '../../../hooks/useClientConfig';
import { gifPickerEnabled } from '../../../utils/gifProvider';
import { normalizeAccentColor, themeDefaultAccentColor } from '../../../utils/themeAccent';
import {
  chromeColorsForRamp,
  DEFAULT_THEME_BASE_COLOR,
  deriveThemeSurfaceRamp,
  normalizeThemeBaseColor,
  THEME_BASE_PRESETS,
} from '../../../utils/themeBase';
import {
  getPlatformIntegrationStatus,
  buildShortcutFailureMessage,
  getPlatformSecretStoreSessionPersistence,
  getPlatformSecretStoreStatusDescription,
  getPlatformSecretStoreStatusLabel,
  isDesktopPlatform,
  platformSessionStore,
  setPlatformShortcuts,
  supportsPlatformGlobalShortcuts,
  type PlatformIntegrationStatus,
  type PlatformSecretStoreStatus,
} from '../../../platform';
import {
  getNativeStoreErrorWarningMessage,
  getSessionBootstrapResult,
  shouldSurfaceNativeStoreErrorWarning,
} from '../../../state/sessionBootstrap';
import { UpdateSettingsTile } from '../../desktop-updater/DesktopUpdaterProvider';

type ThemeSelectorProps = {
  themeNames: Record<string, string>;
  themes: Theme[];
  selected: Theme;
  onSelect: (theme: Theme) => void;
};
const ThemeSelector = as<'div', ThemeSelectorProps>(
  ({ themeNames, themes, selected, onSelect, ...props }, ref) => (
    <Menu {...props} ref={ref}>
      <Box direction="Column" gap="100" style={{ padding: config.space.S100 }}>
        {themes.map((theme) => (
          <MenuItem
            key={theme.id}
            size="300"
            variant={theme.id === selected.id ? 'Primary' : 'Surface'}
            radii="300"
            onClick={() => onSelect(theme)}
          >
            <Text size="T300">{themeNames[theme.id] ?? theme.id}</Text>
          </MenuItem>
        ))}
      </Box>
    </Menu>
  )
);

function SelectTheme({ disabled }: { disabled?: boolean }) {
  const themes = useThemes();
  const themeNames = useThemeNames();
  const [themeId, setThemeId] = useSetting(settingsAtom, 'themeId');
  const [menuCords, setMenuCords] = useState<RectCords>();
  const selectedTheme = themes.find((theme) => theme.id === themeId) ?? LightTheme;

  const handleThemeMenu: MouseEventHandler<HTMLButtonElement> = (evt) => {
    setMenuCords(evt.currentTarget.getBoundingClientRect());
  };

  const handleThemeSelect = (theme: Theme) => {
    setThemeId(theme.id);
    setMenuCords(undefined);
  };

  return (
    <>
      <Button
        size="300"
        variant="Primary"
        outlined
        fill="Soft"
        radii="300"
        after={<Icon size="300" src={Icons.ChevronBottom} />}
        onClick={disabled ? undefined : handleThemeMenu}
        aria-disabled={disabled}
      >
        <Text size="T300">{themeNames[selectedTheme.id] ?? selectedTheme.id}</Text>
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
            <ThemeSelector
              themeNames={themeNames}
              themes={themes}
              selected={selectedTheme}
              onSelect={handleThemeSelect}
            />
          </FocusTrap>
        }
      />
    </>
  );
}

function SystemThemePreferences() {
  const themeKind = useSystemThemeKind();
  const themeNames = useThemeNames();
  const themes = useThemes();
  const [lightThemeId, setLightThemeId] = useSetting(settingsAtom, 'lightThemeId');
  const [darkThemeId, setDarkThemeId] = useSetting(settingsAtom, 'darkThemeId');

  const lightThemes = themes.filter((theme) => theme.kind === ThemeKind.Light);
  const darkThemes = themes.filter((theme) => theme.kind === ThemeKind.Dark);

  const selectedLightTheme = lightThemes.find((theme) => theme.id === lightThemeId) ?? LightTheme;
  const selectedDarkTheme = darkThemes.find((theme) => theme.id === darkThemeId) ?? DarkTheme;

  const [ltCords, setLTCords] = useState<RectCords>();
  const [dtCords, setDTCords] = useState<RectCords>();

  const handleLightThemeMenu: MouseEventHandler<HTMLButtonElement> = (evt) => {
    setLTCords(evt.currentTarget.getBoundingClientRect());
  };
  const handleDarkThemeMenu: MouseEventHandler<HTMLButtonElement> = (evt) => {
    setDTCords(evt.currentTarget.getBoundingClientRect());
  };

  const handleLightThemeSelect = (theme: Theme) => {
    setLightThemeId(theme.id);
    setLTCords(undefined);
  };

  const handleDarkThemeSelect = (theme: Theme) => {
    setDarkThemeId(theme.id);
    setDTCords(undefined);
  };

  return (
    <Box wrap="Wrap" gap="400">
      <SettingTile
        title="Light Theme:"
        after={
          <Chip
            variant={themeKind === ThemeKind.Light ? 'Primary' : 'Secondary'}
            outlined={themeKind === ThemeKind.Light}
            radii="Pill"
            after={<Icon size="200" src={Icons.ChevronBottom} />}
            onClick={handleLightThemeMenu}
          >
            <Text size="B300">{themeNames[selectedLightTheme.id] ?? selectedLightTheme.id}</Text>
          </Chip>
        }
      />
      <PopOut
        anchor={ltCords}
        offset={5}
        position="Bottom"
        align="End"
        content={
          <FocusTrap
            focusTrapOptions={{
              initialFocus: false,
              onDeactivate: () => setLTCords(undefined),
              clickOutsideDeactivates: true,
              isKeyForward: (evt: KeyboardEvent) =>
                evt.key === 'ArrowDown' || evt.key === 'ArrowRight',
              isKeyBackward: (evt: KeyboardEvent) =>
                evt.key === 'ArrowUp' || evt.key === 'ArrowLeft',
              escapeDeactivates: stopPropagation,
            }}
          >
            <ThemeSelector
              themeNames={themeNames}
              themes={lightThemes}
              selected={selectedLightTheme}
              onSelect={handleLightThemeSelect}
            />
          </FocusTrap>
        }
      />
      <SettingTile
        title="Dark Theme:"
        after={
          <Chip
            variant={themeKind === ThemeKind.Dark ? 'Primary' : 'Secondary'}
            outlined={themeKind === ThemeKind.Dark}
            radii="Pill"
            after={<Icon size="200" src={Icons.ChevronBottom} />}
            onClick={handleDarkThemeMenu}
          >
            <Text size="B300">{themeNames[selectedDarkTheme.id] ?? selectedDarkTheme.id}</Text>
          </Chip>
        }
      />
      <PopOut
        anchor={dtCords}
        offset={5}
        position="Bottom"
        align="End"
        content={
          <FocusTrap
            focusTrapOptions={{
              initialFocus: false,
              onDeactivate: () => setDTCords(undefined),
              clickOutsideDeactivates: true,
              isKeyForward: (evt: KeyboardEvent) =>
                evt.key === 'ArrowDown' || evt.key === 'ArrowRight',
              isKeyBackward: (evt: KeyboardEvent) =>
                evt.key === 'ArrowUp' || evt.key === 'ArrowLeft',
              escapeDeactivates: stopPropagation,
            }}
          >
            <ThemeSelector
              themeNames={themeNames}
              themes={darkThemes}
              selected={selectedDarkTheme}
              onSelect={handleDarkThemeSelect}
            />
          </FocusTrap>
        }
      />
    </Box>
  );
}

function PageZoomInput() {
  const [pageZoom, setPageZoom] = useSetting(settingsAtom, 'pageZoom');
  const [currentZoom, setCurrentZoom] = useState(`${pageZoom}`);

  const handleZoomChange: ChangeEventHandler<HTMLInputElement> = (evt) => {
    setCurrentZoom(evt.target.value);
  };

  const handleZoomEnter: KeyboardEventHandler<HTMLInputElement> = (evt) => {
    if (isKeyHotkey('escape', evt)) {
      evt.stopPropagation();
      setCurrentZoom(pageZoom.toString());
    }
    if (
      isKeyHotkey('enter', evt) &&
      'value' in evt.target &&
      typeof evt.target.value === 'string'
    ) {
      const newZoom = parseInt(evt.target.value, 10);
      if (Number.isNaN(newZoom)) return;
      const safeZoom = Math.max(Math.min(newZoom, 150), 75);
      setPageZoom(safeZoom);
      setCurrentZoom(safeZoom.toString());
    }
  };

  return (
    <Input
      style={{ width: toRem(100) }}
      variant={pageZoom === parseInt(currentZoom, 10) ? 'Secondary' : 'Success'}
      size="300"
      radii="300"
      type="number"
      min="75"
      max="150"
      value={currentZoom}
      onChange={handleZoomChange}
      onKeyDown={handleZoomEnter}
      after={<Text size="T300">%</Text>}
      outlined
    />
  );
}

function ThemeRampPreview({ baseColor, kind }: { baseColor: string; kind: ThemeKind }) {
  const chrome = chromeColorsForRamp(deriveThemeSurfaceRamp(baseColor, kind));
  return (
    <Box gap="200" alignItems="Center">
      {(
        [
          ['Rail', chrome.rail],
          ['List', chrome.roomList],
          ['Chat', chrome.chat],
        ] as const
      ).map(([label, fill]) => (
        <Box key={label} direction="Column" gap="100" alignItems="Center">
          <div
            aria-hidden
            style={{
              width: toRem(28),
              height: toRem(36),
              borderRadius: toRem(4),
              backgroundColor: fill,
              boxShadow: 'inset 0 0 0 1px rgba(0,0,0,0.18)',
            }}
          />
          <Text size="T200">{label}</Text>
        </Box>
      ))}
    </Box>
  );
}

function Appearance() {
  const { t } = useTranslation();
  const activeTheme = useTheme();
  const [systemTheme, setSystemTheme] = useSetting(settingsAtom, 'useSystemTheme');
  const [monochromeMode, setMonochromeMode] = useSetting(settingsAtom, 'monochromeMode');
  const [customAccentColor, setCustomAccentColor] = useSetting(settingsAtom, 'customAccentColor');
  const [themeBaseColor, setThemeBaseColor] = useSetting(settingsAtom, 'themeBaseColor');
  const accentColor =
    normalizeAccentColor(customAccentColor) ?? themeDefaultAccentColor(activeTheme.kind);
  const baseColor = normalizeThemeBaseColor(themeBaseColor) ?? DEFAULT_THEME_BASE_COLOR;

  return (
    <Box direction="Column" gap="100">
      <Text size="L400">Appearance</Text>
      <SequenceCard
        className={SequenceCardStyle}
        variant="SurfaceVariant"
        direction="Column"
        gap="400"
      >
        <SettingTile
          title="System Theme"
          description="Choose between light and dark theme based on system preference."
          after={<Switch variant="Primary" value={systemTheme} onChange={setSystemTheme} />}
        />
        {systemTheme && <SystemThemePreferences />}
      </SequenceCard>

      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title="Theme"
          description="Theme to use when system theme is not enabled."
          after={<SelectTheme disabled={systemTheme} />}
        />
      </SequenceCard>

      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title="Monochrome Mode"
          after={<Switch variant="Primary" value={monochromeMode} onChange={setMonochromeMode} />}
        />
      </SequenceCard>
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title={t('modernization.settings.base_color.title', 'Base Color')}
          description={t(
            'modernization.settings.base_color.description',
            'Tint for rail, room list, and chat. Use a swatch, the color well, or paste a hex value. Lightness is mapped to stacked greys, not used as a fill.'
          )}
          after={
            <Box direction="Column" gap="200" style={{ minWidth: toRem(220) }}>
              <ThemeRampPreview baseColor={baseColor} kind={activeTheme.kind} />
              <Box gap="100" wrap="Wrap" alignItems="Center">
                {THEME_BASE_PRESETS.map((preset) => {
                  const selected = baseColor === preset.hex;
                  return (
                    <button
                      key={preset.id}
                      type="button"
                      aria-label={preset.label}
                      aria-pressed={selected}
                      onClick={() => setThemeBaseColor(preset.hex)}
                      style={{
                        width: toRem(22),
                        height: toRem(22),
                        padding: 0,
                        borderRadius: '50%',
                        border: selected
                          ? `2px solid ${baseColor}`
                          : '1px solid rgba(255,255,255,0.18)',
                        background: preset.hex,
                        cursor: 'pointer',
                        outline: selected ? '2px solid currentColor' : 'none',
                        outlineOffset: 2,
                      }}
                    />
                  );
                })}
                <input
                  type="color"
                  value={baseColor}
                  onChange={(evt) => {
                    const next = normalizeThemeBaseColor(evt.currentTarget.value);
                    if (next) setThemeBaseColor(next);
                  }}
                  aria-label={t('modernization.settings.base_color.aria_label', 'Theme base color')}
                  style={{
                    width: toRem(28),
                    height: toRem(28),
                    padding: 0,
                    border: '1px solid rgba(127,127,127,0.35)',
                    borderRadius: toRem(4),
                    background: 'transparent',
                    cursor: 'pointer',
                  }}
                />
              </Box>
              <Box gap="100" alignItems="Center">
                <Input
                  size="300"
                  radii="300"
                  value={baseColor}
                  onChange={(evt) => {
                    const next = normalizeThemeBaseColor(evt.currentTarget.value);
                    if (next) setThemeBaseColor(next);
                  }}
                  aria-label="Theme base color hex"
                  style={{ width: toRem(108) }}
                />
                <Button
                  size="300"
                  radii="300"
                  variant="Secondary"
                  fill="Soft"
                  onClick={() => setThemeBaseColor(undefined)}
                  disabled={!themeBaseColor}
                >
                  <Text size="B300">{t('modernization.settings.base_color.reset', 'Reset')}</Text>
                </Button>
              </Box>
            </Box>
          }
        />
      </SequenceCard>

      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title={t('modernization.settings.accent_color.title', 'Accent Color')}
          description={t(
            'modernization.settings.accent_color.description',
            'Buttons, links, and selected items. The window background is Base Color above. Theme default until you pick a custom color.'
          )}
          after={
            <Box direction="Column" gap="200" style={{ minWidth: toRem(180) }} alignItems="End">
              <Chip as="span" variant="Primary" radii="Pill">
                <Text size="B300">Sample</Text>
              </Chip>
              <Box gap="100" alignItems="Center">
                <input
                  type="color"
                  value={accentColor}
                  onChange={(evt) => {
                    const next = normalizeAccentColor(evt.currentTarget.value);
                    if (next) setCustomAccentColor(next);
                  }}
                  aria-label={t(
                    'modernization.settings.accent_color.aria_label',
                    'Custom accent color'
                  )}
                  style={{
                    width: toRem(28),
                    height: toRem(28),
                    padding: 0,
                    border: '1px solid rgba(127,127,127,0.35)',
                    borderRadius: toRem(4),
                    background: 'transparent',
                    cursor: 'pointer',
                  }}
                />
                <Input
                  size="300"
                  radii="300"
                  value={accentColor}
                  onChange={(evt) => {
                    const next = normalizeAccentColor(evt.currentTarget.value);
                    if (next) setCustomAccentColor(next);
                  }}
                  aria-label="Accent color hex"
                  style={{ width: toRem(108) }}
                />
                <Button
                  size="300"
                  radii="300"
                  variant="Secondary"
                  fill="Soft"
                  onClick={() => setCustomAccentColor(undefined)}
                  disabled={!customAccentColor}
                >
                  <Text size="B300">{t('modernization.settings.accent_color.reset', 'Reset')}</Text>
                </Button>
              </Box>
            </Box>
          }
        />
      </SequenceCard>

      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile title="Page Zoom" after={<PageZoomInput />} />
      </SequenceCard>
    </Box>
  );
}

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
                  <Text size="L400">Day of the Month</Text>
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
                  <Text size="L400">Day of the Week</Text>
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
          tabIndex={-1}
          onClick={handleReset}
          type="reset"
          variant="Secondary"
          size="300"
          radii="300"
        >
          <Icon src={Icons.Cross} size="100" />
        </IconButton>
      ) : (
        <IconButton
          tabIndex={-1}
          onClick={handleOpenMenu}
          type="button"
          variant="Secondary"
          size="300"
          radii="300"
          aria-pressed={!!anchor}
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
          size="400"
          variant={hasChanges ? 'Success' : 'Secondary'}
          fill={hasChanges ? 'Solid' : 'Soft'}
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
        size="300"
        variant="Secondary"
        outlined
        fill="Soft"
        radii="300"
        after={<Icon size="300" src={Icons.ChevronBottom} />}
        onClick={handleMenu}
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
                    size="300"
                    variant={value === item.format ? 'Primary' : 'Surface'}
                    radii="300"
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
        title="Date Format"
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

const DEFAULT_INTEGRATION_STATUS: PlatformIntegrationStatus = {
  platform: 'web',
  desktopEnvironment: 'unknown',
  sessionType: 'unknown',
  distroId: 'unknown',
  distroName: 'unknown',
  distroVersion: 'unknown',
  buildIdentity: 'unknown',
  tray: {
    name: 'Tray',
    ready: false,
    supported: false,
    message: 'Loading integration state.',
  },
  notifications: {
    name: 'Notifications',
    ready: false,
    supported: false,
    message: 'Loading integration state.',
  },
  globalShortcuts: {
    name: 'Global Shortcuts',
    ready: false,
    supported: false,
    message: 'Loading integration state.',
  },
  filePortal: {
    name: 'File Portal',
    ready: false,
    supported: false,
    message: 'Loading integration state.',
  },
  mediaPortal: {
    name: 'Media Portal',
    ready: false,
    supported: false,
    message: 'Loading integration state.',
  },
};

function formatSessionLabel(status: PlatformIntegrationStatus): string {
  const desktopEnvironment = status.desktopEnvironment.toLowerCase();
  const sessionType = status.sessionType.toLowerCase();

  if (desktopEnvironment.includes('kde') && sessionType.includes('wayland')) {
    return 'KDE Plasma Wayland';
  }
  if (desktopEnvironment.includes('kde') && sessionType.includes('x11')) {
    return 'KDE X11';
  }
  if (sessionType.includes('wayland')) return 'Wayland';
  if (sessionType.includes('x11')) return 'X11';
  if (status.sessionType && status.sessionType !== 'unknown') return status.sessionType;
  return status.desktopEnvironment && status.desktopEnvironment !== 'unknown'
    ? status.desktopEnvironment
    : 'Other';
}

function formatDistroLabel(status: PlatformIntegrationStatus): string {
  const versionLabel =
    status.distroVersion && status.distroVersion !== 'unknown' ? ` ${status.distroVersion}` : '';
  const idLabel = status.distroId && status.distroId !== 'unknown' ? ` (${status.distroId})` : '';
  const distroName =
    status.distroName && status.distroName !== 'unknown' ? status.distroName : 'Unknown';
  return `${distroName}${idLabel}${versionLabel}`;
}

function formatCheckLabel(ready: boolean, supported: boolean): string {
  if (!supported) return 'Unavailable';
  return ready ? 'Ready' : 'Not ready';
}

function buildDiagnosticsPayload(status: PlatformIntegrationStatus): string {
  return JSON.stringify(
    {
      platform: status.platform,
      desktopEnvironment: status.desktopEnvironment,
      sessionType: status.sessionType,
      distro: {
        id: status.distroId,
        name: status.distroName,
        version: status.distroVersion,
      },
      buildIdentity: status.buildIdentity,
      checks: {
        tray: status.tray,
        notifications: status.notifications,
        globalShortcuts: status.globalShortcuts,
        filePortal: status.filePortal,
        mediaPortal: status.mediaPortal,
      },
    },
    undefined,
    2
  );
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
  const hasValue =
    normalizedShowShortcut.length > 0 &&
    normalizedLaterShortcut.length > 0 &&
    normalizedNotificationsShortcut.length > 0;

  const uniqueShortcuts = new Set([
    normalizedShowShortcut,
    normalizedLaterShortcut,
    normalizedNotificationsShortcut,
  ]);
  const hasDuplicate = uniqueShortcuts.size !== 3;

  const hasChanges =
    normalizedShowShortcut !== storedShowShortcut ||
    normalizedLaterShortcut !== storedLaterShortcut ||
    normalizedNotificationsShortcut !== storedNotificationsShortcut;

  const applyError = (() => {
    if (!hasValue) {
      return t(
        'modernization.settings.desktop_shortcuts.empty',
        'All shortcut values are required.'
      );
    }
    if (hasDuplicate) {
      return t('modernization.settings.desktop_shortcuts.duplicate', 'Shortcuts must be unique.');
    }
    return undefined;
  })();

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

  return (
    <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
      <SettingTile
        title={t('modernization.settings.desktop_shortcuts.title', 'Desktop Shortcuts')}
        description={
          shortcutsSupported
            ? t(
                'modernization.settings.desktop_shortcuts.description',
                'Open frequently used screens from anywhere with desktop shortcuts.'
              )
            : t(
                'modernization.settings.desktop_shortcuts.unavailable',
                'Desktop shortcuts are only available in the desktop app.'
              )
        }
        after={
          <Text size="T200" priority="300">
            {shortcutsSupported
              ? t('modernization.settings.desktop_shortcuts.platform', 'Desktop only')
              : t('modernization.settings.desktop_shortcuts.platform_disabled', 'Unavailable')}
          </Text>
        }
      />
      <Box as="form" onSubmit={handleSave} gap="200" direction="Column" style={{ padding: 12 }}>
        <Input
          aria-label={t(
            'modernization.settings.desktop_shortcuts.show_label',
            'Show main window shortcut'
          )}
          value={showShortcut}
          onChange={(evt) => {
            setError(undefined);
            setShowShortcut(evt.currentTarget.value);
          }}
          disabled={!shortcutsSupported}
          placeholder="CmdOrCtrl+Shift+C"
        />
        <Input
          aria-label={t(
            'modernization.settings.desktop_shortcuts.later_label',
            'Open Later shortcut'
          )}
          value={laterShortcut}
          onChange={(evt) => {
            setError(undefined);
            setLaterShortcut(evt.currentTarget.value);
          }}
          disabled={!shortcutsSupported}
          placeholder="CmdOrCtrl+Shift+L"
        />
        <Input
          aria-label={t(
            'modernization.settings.desktop_shortcuts.notifications_label',
            'Open Notifications shortcut'
          )}
          value={notificationsShortcut}
          onChange={(evt) => {
            setError(undefined);
            setNotificationsShortcut(evt.currentTarget.value);
          }}
          disabled={!shortcutsSupported}
          placeholder="CmdOrCtrl+Shift+N"
        />
        {(applyError || error) && (
          <Text size="T200" priority="500">
            {error ?? applyError}
          </Text>
        )}
        <Button
          size="300"
          variant={hasChanges ? 'Primary' : 'Secondary'}
          fill={hasChanges ? 'Solid' : 'None'}
          disabled={!shortcutsSupported || saving || !hasChanges || !!applyError}
          type="submit"
        >
          <Text size="B300">
            {saving
              ? t('modernization.settings.desktop_shortcuts.saving', 'Saving…')
              : t('modernization.settings.desktop_shortcuts.apply', 'Apply shortcuts')}
          </Text>
        </Button>
      </Box>
    </SequenceCard>
  );
}

function SecretStoreSection() {
  const { t } = useTranslation();
  const [status, setStatus] = useState<PlatformSecretStoreStatus>();
  const [nativeStoreError, setNativeStoreError] = useState(
    () => getSessionBootstrapResult().nativeStoreError
  );

  useEffect(() => {
    let active = true;

    platformSessionStore.getStatus().then((nextStatus) => {
      if (!active) return;
      setStatus(nextStatus);
      setNativeStoreError(getSessionBootstrapResult().nativeStoreError);
    });

    return () => {
      active = false;
    };
  }, []);

  if (!isDesktopPlatform()) return null;

  const persistence = status ? getPlatformSecretStoreSessionPersistence(status) : undefined;
  const badgeVariant =
    persistence === 'persistent'
      ? 'Success'
      : persistence === 'session-scoped'
      ? 'Warning'
      : status
      ? 'Critical'
      : 'Secondary';
  const statusLabel = status ? getPlatformSecretStoreStatusLabel(status) : 'Checking';
  const details = status ? getPlatformSecretStoreStatusDescription(status) : statusLabel;
  const showNativeStoreErrorWarning = shouldSurfaceNativeStoreErrorWarning(nativeStoreError, true);
  const nativeStoreErrorWarning = showNativeStoreErrorWarning
    ? t(
        'modernization.settings.secret_store.native_store_error',
        getNativeStoreErrorWarningMessage()
      )
    : undefined;

  return (
    <Box direction="Column" gap="100">
      <Text size="L400">{t('modernization.settings.secret_store.title', 'Session Storage')}</Text>
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title={t('modernization.settings.secret_store.status_title', 'Native Session Store')}
          description={details}
          after={
            <Chip variant={badgeVariant} radii="Pill" outlined>
              <Text size="B300">{statusLabel}</Text>
            </Chip>
          }
        />
        {nativeStoreErrorWarning && (
          <Box style={{ padding: 12, paddingTop: 0 }}>
            <Text size="T200" priority="500">
              {nativeStoreErrorWarning}
            </Text>
          </Box>
        )}
      </SequenceCard>
    </Box>
  );
}

function DesktopIntegrationSection() {
  const [status, setStatus] = useState<PlatformIntegrationStatus>(DEFAULT_INTEGRATION_STATUS);
  const [copyError, setCopyError] = useState(false);
  const [copySuccess, setCopySuccess] = useState(false);

  useEffect(() => {
    let cancelled = false;
    const load = async () => {
      const nextStatus = await getPlatformIntegrationStatus();
      if (!cancelled) setStatus(nextStatus);
    };
    load();
    return () => {
      cancelled = true;
    };
  }, []);

  const copyDiagnostics = async () => {
    try {
      await navigator.clipboard.writeText(buildDiagnosticsPayload(status));
      setCopyError(false);
      setCopySuccess(true);
    } catch {
      setCopyError(true);
      setCopySuccess(false);
    }
  };

  return (
    <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
      <SettingTile
        title="Desktop Integration"
        description="Current session diagnostics for tray, shortcuts, and portal readiness."
      />
      <Box direction="Column" gap="200" style={{ padding: 12 }}>
        <Box direction="Column" gap="100">
          <Text size="B400">Session</Text>
          <Text size="T300" priority="400">
            {formatSessionLabel(status)}
          </Text>
        </Box>
        <Box direction="Column" gap="100">
          <Text size="B400">Distribution</Text>
          <Text size="T300" priority="400">
            {formatDistroLabel(status)}
          </Text>
        </Box>
        <Box direction="Column" gap="100">
          <Text size="T300" priority="500">
            {`Tray: ${formatCheckLabel(status.tray.ready, status.tray.supported)}`}
          </Text>
          <Text size="T200" priority="300">
            {status.tray.message}
          </Text>
          <Text size="T300" priority="500">
            {`Notifications: ${formatCheckLabel(
              status.notifications.ready,
              status.notifications.supported
            )}`}
          </Text>
          <Text size="T200" priority="300">
            {status.notifications.message}
          </Text>
          <Text size="T300" priority="500">
            {`Global Shortcuts: ${formatCheckLabel(
              status.globalShortcuts.ready,
              status.globalShortcuts.supported
            )}`}
          </Text>
          <Text size="T200" priority="300">
            {status.globalShortcuts.message}
          </Text>
          <Text size="T300" priority="500">
            {`File Portal: ${formatCheckLabel(
              status.filePortal.ready,
              status.filePortal.supported
            )}`}
          </Text>
          <Text size="T200" priority="300">
            {status.filePortal.message}
          </Text>
          <Text size="T300" priority="500">
            {`Media Portal: ${formatCheckLabel(
              status.mediaPortal.ready,
              status.mediaPortal.supported
            )}`}
          </Text>
          <Text size="T200" priority="300">
            {status.mediaPortal.message}
          </Text>
        </Box>
        <Button size="300" variant="Secondary" fill="Soft" radii="300" onClick={copyDiagnostics}>
          <Text size="B300">Copy diagnostics</Text>
        </Button>
        {copyError && <Text size="T200">Clipboard copy failed.</Text>}
        {copySuccess && <Text size="T200">Diagnostics copied.</Text>}
      </Box>
    </SequenceCard>
  );
}

function SoftwareUpdatesSection() {
  if (!isDesktopPlatform()) return null;

  return (
    <Box direction="Column" gap="100">
      <Text size="L400">Software Updates</Text>
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <UpdateSettingsTile />
      </SequenceCard>
    </Box>
  );
}

function DateAndTime() {
  const [hour24Clock, setHour24Clock] = useSetting(settingsAtom, 'hour24Clock');

  return (
    <Box direction="Column" gap="100">
      <Text size="L400">Date & Time</Text>
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title="24-Hour Time Format"
          after={<Switch variant="Primary" value={hour24Clock} onChange={setHour24Clock} />}
        />
      </SequenceCard>

      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SelectDateFormat />
      </SequenceCard>
    </Box>
  );
}

function Editor() {
  const [enterForNewline, setEnterForNewline] = useSetting(settingsAtom, 'enterForNewline');
  const [isMarkdown, setIsMarkdown] = useSetting(settingsAtom, 'isMarkdown');
  const [hideActivity, setHideActivity] = useSetting(settingsAtom, 'hideActivity');

  return (
    <Box direction="Column" gap="100">
      <Text size="L400">Editor</Text>
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title="ENTER for Newline"
          description={`Use ${
            isMacOS() ? KeySymbol.Command : 'Ctrl'
          } + ENTER to send message and ENTER for newline.`}
          after={<Switch variant="Primary" value={enterForNewline} onChange={setEnterForNewline} />}
        />
      </SequenceCard>
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title="Markdown Formatting"
          after={<Switch variant="Primary" value={isMarkdown} onChange={setIsMarkdown} />}
        />
      </SequenceCard>
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title="Hide Typing & Read Receipts"
          description="Turn off both typing status and read receipts to keep your activity private."
          after={<Switch variant="Primary" value={hideActivity} onChange={setHideActivity} />}
        />
      </SequenceCard>
    </Box>
  );
}

function SelectMessageLayout() {
  const [menuCords, setMenuCords] = useState<RectCords>();
  const [messageLayout, setMessageLayout] = useSetting(settingsAtom, 'messageLayout');
  const messageLayoutItems = useMessageLayoutItems();

  const handleMenu: MouseEventHandler<HTMLButtonElement> = (evt) => {
    setMenuCords(evt.currentTarget.getBoundingClientRect());
  };

  const handleSelect = (layout: MessageLayout) => {
    setMessageLayout(layout);
    setMenuCords(undefined);
  };

  return (
    <>
      <Button
        size="300"
        variant="Secondary"
        outlined
        fill="Soft"
        radii="300"
        after={<Icon size="300" src={Icons.ChevronBottom} />}
        onClick={handleMenu}
      >
        <Text size="T300">
          {messageLayoutItems.find((i) => i.layout === messageLayout)?.name ?? messageLayout}
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
                {messageLayoutItems.map((item) => (
                  <MenuItem
                    key={item.layout}
                    size="300"
                    variant={messageLayout === item.layout ? 'Primary' : 'Surface'}
                    radii="300"
                    onClick={() => handleSelect(item.layout)}
                  >
                    <Text size="T300">{item.name}</Text>
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

function SelectMessageSpacing() {
  const [menuCords, setMenuCords] = useState<RectCords>();
  const [messageSpacing, setMessageSpacing] = useSetting(settingsAtom, 'messageSpacing');
  const messageSpacingItems = useMessageSpacingItems();

  const handleMenu: MouseEventHandler<HTMLButtonElement> = (evt) => {
    setMenuCords(evt.currentTarget.getBoundingClientRect());
  };

  const handleSelect = (layout: MessageSpacing) => {
    setMessageSpacing(layout);
    setMenuCords(undefined);
  };

  return (
    <>
      <Button
        size="300"
        variant="Secondary"
        outlined
        fill="Soft"
        radii="300"
        after={<Icon size="300" src={Icons.ChevronBottom} />}
        onClick={handleMenu}
      >
        <Text size="T300">
          {messageSpacingItems.find((i) => i.spacing === messageSpacing)?.name ?? messageSpacing}
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
                {messageSpacingItems.map((item) => (
                  <MenuItem
                    key={item.spacing}
                    size="300"
                    variant={messageSpacing === item.spacing ? 'Primary' : 'Surface'}
                    radii="300"
                    onClick={() => handleSelect(item.spacing)}
                  >
                    <Text size="T300">{item.name}</Text>
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

function Messages() {
  const { t } = useTranslation();
  const clientConfig = useClientConfig();
  const [legacyUsernameColor, setLegacyUsernameColor] = useSetting(
    settingsAtom,
    'legacyUsernameColor'
  );
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
  const [showHiddenEvents, setShowHiddenEvents] = useSetting(settingsAtom, 'showHiddenEvents');
  const gifProviderAvailable = gifPickerEnabled(clientConfig.gifPicker);

  return (
    <Box direction="Column" gap="100">
      <Text size="L400">Messages</Text>
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile title="Message Layout" after={<SelectMessageLayout />} />
      </SequenceCard>
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile title="Message Spacing" after={<SelectMessageSpacing />} />
      </SequenceCard>
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title="Legacy Username Color"
          after={
            <Switch
              variant="Primary"
              value={legacyUsernameColor}
              onChange={setLegacyUsernameColor}
            />
          }
        />
      </SequenceCard>
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title="Hide Membership Change"
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
          title="Hide Profile Change"
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
          title="Disable Media Auto Load"
          after={
            <Switch
              variant="Primary"
              value={!mediaAutoLoad}
              onChange={(v) => setMediaAutoLoad(!v)}
            />
          }
        />
      </SequenceCard>
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title={t('modernization.settings.gif_search.title', 'GIF Search')}
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
          title="Show Hidden Events"
          after={
            <Switch variant="Primary" value={showHiddenEvents} onChange={setShowHiddenEvents} />
          }
        />
      </SequenceCard>
    </Box>
  );
}

type GeneralProps = {
  requestClose: () => void;
};
export function General({ requestClose }: GeneralProps) {
  return (
    <Page>
      <PageHeader outlined={false}>
        <Box grow="Yes" gap="200">
          <Box grow="Yes" alignItems="Center" gap="200">
            <Text size="H3" truncate>
              General
            </Text>
          </Box>
          <Box shrink="No">
            <IconButton onClick={requestClose} variant="Surface">
              <Icon src={Icons.Cross} />
            </IconButton>
          </Box>
        </Box>
      </PageHeader>
      <Box grow="Yes">
        <Scroll hideTrack visibility="Hover">
          <PageContent>
            <Box direction="Column" gap="700">
              <Appearance />
              <DateAndTime />
              <Editor />
              <Messages />
              <SecretStoreSection />
              <SoftwareUpdatesSection />
              <DesktopShortcutsSection />
              <DesktopIntegrationSection />
            </Box>
          </PageContent>
        </Scroll>
      </Box>
    </Page>
  );
}
