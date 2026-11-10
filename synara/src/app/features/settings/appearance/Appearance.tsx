import React, {
  ChangeEventHandler,
  KeyboardEventHandler,
  MouseEventHandler,
  useEffect,
  useState,
} from 'react';
import {
  as,
  Box,
  Button,
  Chip,
  config,
  Icon,
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
import { isKeyHotkey } from 'is-hotkey';
import FocusTrap from '../../../components/FocusTrap';
import { useTranslation } from 'react-i18next';
import { SequenceCard } from '../../../components/sequence-card';
import { useSetting } from '../../../state/hooks/settings';
import {
  desktopPlatformSettingsAtom,
  MessageLayout,
  MessageSpacing,
  settingsAtom,
  TilingModeSetting,
} from '../../../state/settings';
import { SettingTile } from '../../../components/setting-tile';
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
import {
  SequenceCardStyle,
  SettingsQuietControl,
  SettingsThemeSwatch,
  SettingsThemeSwatchOutline,
} from '../styles.css';
import { isNativeMatrixSession } from '../../verification/nativeVerification';
import { normalizeAccentColor, themeDefaultAccentColor } from '../../../utils/themeAccent';
import { MESSAGE_TEXT_TONES, type MessageTextTone } from '../../../utils/messageTextTone';
import {
  THREAD_DISPLAY_LABELS,
  THREAD_DISPLAY_MODES,
  ThreadDisplayMode,
} from '../../../utils/threadDisplay';
import {
  chromeColorsForRamp,
  DEFAULT_THEME_BASE_COLOR,
  deriveThemeSurfaceRamp,
  normalizeThemeBaseColor,
  THEME_BASE_PRESETS,
} from '../../../utils/themeBase';
import { SettingsPage, SettingsSection } from '../../../components/settings-layout';
import { useSystemPalette } from '../../../hooks/useSystemPalette';
import { externalPaletteSourceLabel } from '../../../utils/themeExternalPalette';
import { isLinuxDesktopShell } from '../../../hooks/useTilingMode';
import { getDesktopWindowEnvironment } from '../../../utils/desktop';
import {
  DESKTOP_RENDERING_UNAVAILABLE,
  getDesktopRenderingStatus,
  hardwareAccelerationView,
  setDesktopRenderingMode,
} from '../../../utils/desktopRendering';
import {
  InterfaceFontChoice,
  MonospaceFontChoice,
  sanitizeFontFamilyList,
} from '../../../utils/interfaceFonts';

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
            className={SettingsQuietControl}
            size="300"
            variant="Surface"
            radii="300"
            aria-pressed={theme.id === selected.id}
            after={theme.id === selected.id ? <Icon size="100" src={Icons.Check} /> : undefined}
            onClick={() => onSelect(theme)}
          >
            <Text size="T300">{themeNames[theme.id] ?? theme.id}</Text>
          </MenuItem>
        ))}
      </Box>
    </Menu>
  )
);

function SelectTheme({ kind, disabled }: { kind: ThemeKind; disabled?: boolean }) {
  const allThemes = useThemes();
  const themes = allThemes.filter((theme) => theme.kind === kind);
  const themeNames = useThemeNames();
  const [themeId, setThemeId] = useSetting(settingsAtom, 'themeId');
  const [menuCords, setMenuCords] = useState<RectCords>();
  const selectedTheme =
    themes.find((theme) => theme.id === themeId) ??
    (kind === ThemeKind.Dark ? DarkTheme : LightTheme);

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
        className={SettingsQuietControl}
        size="300"
        variant="Secondary"
        outlined
        fill="None"
        radii="300"
        after={<Icon size="300" src={Icons.ChevronBottom} />}
        onClick={disabled ? undefined : handleThemeMenu}
        disabled={disabled}
        aria-haspopup="menu"
        aria-expanded={menuCords !== undefined}
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
    <>
      <SettingTile
        title="Light theme"
        description={
          themeKind === ThemeKind.Light
            ? 'Used while your system is in light mode. Active now.'
            : 'Used while your system is in light mode.'
        }
        after={
          <Button
            className={SettingsQuietControl}
            size="300"
            variant="Secondary"
            outlined
            fill="None"
            radii="300"
            after={<Icon size="300" src={Icons.ChevronBottom} />}
            onClick={handleLightThemeMenu}
            aria-haspopup="menu"
            aria-expanded={ltCords !== undefined}
          >
            <Text size="T300">{themeNames[selectedLightTheme.id] ?? selectedLightTheme.id}</Text>
          </Button>
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
        title="Dark theme"
        description={
          themeKind === ThemeKind.Dark
            ? 'Used while your system is in dark mode. Active now.'
            : 'Used while your system is in dark mode.'
        }
        after={
          <Button
            className={SettingsQuietControl}
            size="300"
            variant="Secondary"
            outlined
            fill="None"
            radii="300"
            after={<Icon size="300" src={Icons.ChevronBottom} />}
            onClick={handleDarkThemeMenu}
            aria-haspopup="menu"
            aria-expanded={dtCords !== undefined}
          >
            <Text size="T300">{themeNames[selectedDarkTheme.id] ?? selectedDarkTheme.id}</Text>
          </Button>
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
    </>
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
      const safeZoom = Math.max(Math.min(newZoom, 200), 75);
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
      aria-label="Page zoom percentage"
      min="75"
      max="200"
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
          // The former rail's token: now the thread, members and other side panels.
          ['Panels', chrome.rail],
          ['List', chrome.roomList],
          ['Chat', chrome.chat],
        ] as const
      ).map(([label, fill]) => (
        <Box key={label} direction="Column" gap="100" alignItems="Center">
          <div
            aria-hidden
            className={SettingsThemeSwatchOutline}
            style={{
              width: toRem(28),
              height: toRem(36),
              borderRadius: config.radii.R300,
              backgroundColor: fill,
            }}
          />
          <Text size="T200" priority="300">
            {label}
          </Text>
        </Box>
      ))}
    </Box>
  );
}

type AppearanceMode = 'system' | 'light' | 'dark';

const APPEARANCE_MODES: Array<{ mode: AppearanceMode; label: string }> = [
  { mode: 'system', label: 'System' },
  { mode: 'light', label: 'Light' },
  { mode: 'dark', label: 'Dark' },
];

/** One quiet segmented group, the same pattern as Thread display. */
function SegmentedChoice<T extends string>({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: T;
  options: Array<{ value: T; label: string }>;
  onChange: (value: T) => void;
}) {
  return (
    <Box gap="100" role="group" aria-label={label}>
      {options.map((option) => (
        <Button
          key={option.value}
          className={SettingsQuietControl}
          size="300"
          radii="300"
          variant="Secondary"
          fill={value === option.value ? 'Soft' : 'None'}
          aria-pressed={value === option.value}
          before={
            <Icon
              size="100"
              src={Icons.Check}
              aria-hidden
              style={{ visibility: value === option.value ? 'visible' : 'hidden' }}
            />
          }
          onClick={() => onChange(option.value)}
        >
          <Text size="B300">{option.label}</Text>
        </Button>
      ))}
    </Box>
  );
}

/** Swatches, the colour well, the hex field and Reset, left-aligned under the row copy. */
function ColorControls({
  value,
  presets,
  onChange,
  onReset,
  canReset,
  wellLabel,
  hexLabel,
  resetLabel,
}: {
  value: string;
  presets?: ReadonlyArray<{ id: string; label: string; hex: string }>;
  onChange: (hex: string) => void;
  onReset: () => void;
  canReset: boolean;
  wellLabel: string;
  hexLabel: string;
  resetLabel: string;
}) {
  return (
    <Box gap="200" wrap="Wrap" alignItems="Center">
      {presets && presets.length > 0 && (
        <Box gap="100" wrap="Wrap" alignItems="Center">
          {presets.map((preset) => {
            const selected = value === preset.hex;
            return (
              <button
                key={preset.id}
                type="button"
                className={SettingsThemeSwatch}
                aria-label={preset.label}
                aria-pressed={selected}
                onClick={() => onChange(preset.hex)}
                style={{ background: preset.hex }}
              />
            );
          })}
        </Box>
      )}
      <Box gap="100" alignItems="Center">
        <input
          type="color"
          className={SettingsThemeSwatchOutline}
          value={value}
          onChange={(evt) => onChange(evt.currentTarget.value)}
          aria-label={wellLabel}
          style={{
            width: toRem(28),
            height: toRem(28),
            padding: 0,
            border: 0,
            borderRadius: config.radii.R300,
            background: 'transparent',
            cursor: 'pointer',
          }}
        />
        <Input
          size="300"
          radii="300"
          variant="Secondary"
          outlined
          value={value}
          onChange={(evt) => onChange(evt.currentTarget.value)}
          aria-label={hexLabel}
          style={{ width: toRem(108), fontFamily: 'var(--font-mono)' }}
        />
        <Button
          className={SettingsQuietControl}
          size="300"
          radii="300"
          variant="Secondary"
          fill="None"
          onClick={onReset}
          disabled={!canReset}
        >
          <Text size="B300">{resetLabel}</Text>
        </Button>
      </Box>
    </Box>
  );
}

/**
 * Offered while the desktop shares a palette, and kept while it is on so it
 * can be turned off after the palette goes away.
 */
function FollowSystemPaletteSetting() {
  const { palette } = useSystemPalette();
  const [followSystemPalette, setFollowSystemPalette] = useSetting(
    settingsAtom,
    'followSystemPalette'
  );
  if (!palette && !followSystemPalette) return null;

  let description: string;
  if (!palette) {
    description =
      'Your desktop has no theme colors to share right now, so the theme above applies.';
  } else {
    description = `Use the background, text and accent colors of ${externalPaletteSourceLabel(palette.source)}. Synara adjusts them where text would be hard to read.`;
  }

  return (
    <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
      <SettingTile
        title="Follow system theme colors"
        description={description}
        after={
          <Switch variant="Primary" value={followSystemPalette} onChange={setFollowSystemPalette} />
        }
      />
    </SequenceCard>
  );
}

function Appearance() {
  const { t } = useTranslation();
  const activeTheme = useTheme();
  const themes = useThemes();
  const [systemTheme, setSystemTheme] = useSetting(settingsAtom, 'useSystemTheme');
  const [themeId, setThemeId] = useSetting(settingsAtom, 'themeId');
  const [lightThemeId] = useSetting(settingsAtom, 'lightThemeId');
  const [darkThemeId] = useSetting(settingsAtom, 'darkThemeId');
  const [monochromeMode, setMonochromeMode] = useSetting(settingsAtom, 'monochromeMode');
  const [customAccentColor, setCustomAccentColor] = useSetting(settingsAtom, 'customAccentColor');
  const [themeBaseColor, setThemeBaseColor] = useSetting(settingsAtom, 'themeBaseColor');
  const accentColor =
    normalizeAccentColor(customAccentColor) ?? themeDefaultAccentColor(activeTheme.kind);
  const baseColor = normalizeThemeBaseColor(themeBaseColor) ?? DEFAULT_THEME_BASE_COLOR;

  const chosenTheme = themes.find((theme) => theme.id === themeId) ?? LightTheme;
  const mode: AppearanceMode = systemTheme
    ? 'system'
    : chosenTheme.kind === ThemeKind.Dark
      ? 'dark'
      : 'light';

  const chooseMode = (next: AppearanceMode) => {
    if (next === 'system') {
      setSystemTheme(true);
      return;
    }
    setSystemTheme(false);
    const kind = next === 'dark' ? ThemeKind.Dark : ThemeKind.Light;
    if (chosenTheme.kind !== kind) {
      // Keep the theme already picked for that side of the system switch.
      const preferred = next === 'dark' ? darkThemeId : lightThemeId;
      const fallback = next === 'dark' ? DarkTheme : LightTheme;
      setThemeId(themes.find((theme) => theme.id === preferred)?.id ?? fallback.id);
    }
  };

  return (
    <SettingsSection title="Theme">
      <SequenceCard
        className={SequenceCardStyle}
        variant="SurfaceVariant"
        direction="Column"
        gap="400"
      >
        <SettingTile
          title="Appearance"
          description="Follow the system setting, or keep Synara light or dark."
          after={
            <SegmentedChoice
              label="Appearance"
              value={mode}
              options={APPEARANCE_MODES.map(({ mode: value, label }) => ({ value, label }))}
              onChange={chooseMode}
            />
          }
        />
        {mode === 'system' ? (
          <SystemThemePreferences />
        ) : (
          <SettingTile
            title="Theme"
            description={mode === 'dark' ? 'Which dark theme to use.' : 'Which light theme to use.'}
            after={<SelectTheme kind={mode === 'dark' ? ThemeKind.Dark : ThemeKind.Light} />}
          />
        )}
      </SequenceCard>
      <FollowSystemPaletteSetting />

      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title="Monochrome mode"
          description="Render the interface in greyscale, keeping only the accent color."
          after={<Switch variant="Primary" value={monochromeMode} onChange={setMonochromeMode} />}
        />
      </SequenceCard>
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title={t('modernization.settings.base_color.title', 'Base color')}
          description={t(
            'modernization.settings.base_color.description',
            'Tints the side panels, room list and chat. Lightness is mapped to the stacked greys, not used as a fill.'
          )}
          after={<ThemeRampPreview baseColor={baseColor} kind={activeTheme.kind} />}
        >
          <Box style={{ paddingTop: config.space.S100 }}>
            <ColorControls
              value={baseColor}
              presets={THEME_BASE_PRESETS}
              onChange={(hex) => {
                const next = normalizeThemeBaseColor(hex);
                if (next) setThemeBaseColor(next);
              }}
              onReset={() => setThemeBaseColor(undefined)}
              canReset={!!themeBaseColor}
              wellLabel={t('modernization.settings.base_color.aria_label', 'Theme base color')}
              hexLabel="Theme base color hex"
              resetLabel={t('modernization.settings.base_color.reset', 'Reset')}
            />
          </Box>
        </SettingTile>
      </SequenceCard>

      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title={t('modernization.settings.accent_color.title', 'Accent color')}
          description={t(
            'modernization.settings.accent_color.description',
            'Buttons, links and selected items. Theme default until you pick a color.'
          )}
          after={
            <Chip as="span" variant="Primary" radii="Pill">
              <Text size="B300">Sample</Text>
            </Chip>
          }
        >
          <Box style={{ paddingTop: config.space.S100 }}>
            <ColorControls
              value={accentColor}
              onChange={(hex) => {
                const next = normalizeAccentColor(hex);
                if (next) setCustomAccentColor(next);
              }}
              onReset={() => setCustomAccentColor(undefined)}
              canReset={!!customAccentColor}
              wellLabel={t('modernization.settings.accent_color.aria_label', 'Custom accent color')}
              hexLabel="Accent color hex"
              resetLabel={t('modernization.settings.accent_color.reset', 'Reset')}
            />
          </Box>
        </SettingTile>
      </SequenceCard>
    </SettingsSection>
  );
}

function ThreadDisplaySetting() {
  const [threadDisplay, setThreadDisplay] = useSetting(settingsAtom, 'threadDisplay');
  const [, setThreadDisplayChosen] = useSetting(settingsAtom, 'threadDisplayChosen');
  const chooseThreadDisplay = (mode: ThreadDisplayMode) => {
    setThreadDisplayChosen(true);
    setThreadDisplay(mode);
  };

  return (
    <SettingTile
      title="Thread display"
      description="Open threads inline beneath their first message, in a side panel beside the room, or in place of the room."
      after={
        <Box gap="100" role="group" aria-label="Thread display">
          {THREAD_DISPLAY_MODES.map((mode) => (
            <Button
              key={mode}
              className={SettingsQuietControl}
              size="300"
              radii="300"
              variant="Secondary"
              fill={threadDisplay === mode ? 'Soft' : 'None'}
              aria-pressed={threadDisplay === mode}
              before={
                <Icon
                  size="100"
                  src={Icons.Check}
                  aria-hidden
                  style={{ visibility: threadDisplay === mode ? 'visible' : 'hidden' }}
                />
              }
              onClick={() => chooseThreadDisplay(mode)}
            >
              <Text size="B300">{THREAD_DISPLAY_LABELS[mode]}</Text>
            </Button>
          ))}
        </Box>
      }
    />
  );
}

/** A family list, applied on Enter or when the field loses focus; Escape restores. */
function FontFamilyField({
  value,
  onCommit,
  label,
  placeholder,
}: {
  value: string;
  onCommit: (family: string) => void;
  label: string;
  placeholder: string;
}) {
  const [draft, setDraft] = useState(value);
  useEffect(() => setDraft(value), [value]);

  const commit = () => {
    const next = sanitizeFontFamilyList(draft);
    setDraft(next);
    if (next !== value) onCommit(next);
  };

  return (
    <Input
      size="300"
      radii="300"
      variant="Secondary"
      outlined
      value={draft}
      aria-label={label}
      placeholder={placeholder}
      spellCheck={false}
      autoComplete="off"
      maxLength={280}
      onChange={(evt) => setDraft(evt.currentTarget.value)}
      onBlur={commit}
      onKeyDown={(evt) => {
        if (isKeyHotkey('escape', evt)) {
          evt.stopPropagation();
          setDraft(value);
        }
        if (isKeyHotkey('enter', evt)) commit();
      }}
      style={{ width: toRem(280) }}
    />
  );
}

const INTERFACE_FONT_OPTIONS: Array<{ value: InterfaceFontChoice; label: string }> = [
  { value: 'default', label: 'Default' },
  { value: 'monospace', label: 'Monospace' },
  { value: 'custom', label: 'Custom' },
];

const MONOSPACE_FONT_OPTIONS: Array<{ value: MonospaceFontChoice; label: string }> = [
  { value: 'default', label: 'Default' },
  { value: 'custom', label: 'Custom' },
];

function FontSettings() {
  const [interfaceFont, setInterfaceFont] = useSetting(settingsAtom, 'interfaceFont');
  const [interfaceFontFamily, setInterfaceFontFamily] = useSetting(
    settingsAtom,
    'interfaceFontFamily'
  );
  const [monospaceFont, setMonospaceFont] = useSetting(settingsAtom, 'monospaceFont');
  const [monospaceFontFamily, setMonospaceFontFamily] = useSetting(
    settingsAtom,
    'monospaceFontFamily'
  );

  return (
    <>
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title="Interface font"
          description="The font for menus, lists and messages. Custom takes font families installed on this device, separated by commas; the default font covers anything they lack."
          after={
            <SegmentedChoice
              label="Interface font"
              value={interfaceFont}
              options={INTERFACE_FONT_OPTIONS}
              onChange={setInterfaceFont}
            />
          }
        >
          {interfaceFont === 'custom' && (
            <Box style={{ paddingTop: config.space.S100 }}>
              <FontFamilyField
                value={interfaceFontFamily}
                onCommit={setInterfaceFontFamily}
                label="Interface font family"
                placeholder="For example: Inter, Noto Sans"
              />
            </Box>
          )}
        </SettingTile>
      </SequenceCard>
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title="Monospace font"
          description="The font for code and other fixed-width text. Custom takes font families installed on this device, separated by commas."
          after={
            <SegmentedChoice
              label="Monospace font"
              value={monospaceFont}
              options={MONOSPACE_FONT_OPTIONS}
              onChange={setMonospaceFont}
            />
          }
        >
          {monospaceFont === 'custom' && (
            <Box style={{ paddingTop: config.space.S100 }}>
              <FontFamilyField
                value={monospaceFontFamily}
                onCommit={setMonospaceFontFamily}
                label="Monospace font family"
                placeholder="For example: JetBrains Mono"
              />
            </Box>
          )}
        </SettingTile>
      </SequenceCard>
    </>
  );
}

function TextAndZoom() {
  const [messageTextTone, setMessageTextTone] = useSetting(settingsAtom, 'messageTextTone');

  return (
    <SettingsSection title="Text">
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title="Message text"
          description="Adjust message prose brightness without changing navigation, metadata, or code colors."
          after={
            <Box gap="100" role="group" aria-label="Message text brightness">
              {MESSAGE_TEXT_TONES.map((tone) => (
                <Button
                  key={tone}
                  className={SettingsQuietControl}
                  size="300"
                  radii="300"
                  variant="Secondary"
                  fill={messageTextTone === tone ? 'Soft' : 'None'}
                  aria-pressed={messageTextTone === tone}
                  before={
                    <Icon
                      size="100"
                      src={Icons.Check}
                      aria-hidden
                      style={{ visibility: messageTextTone === tone ? 'visible' : 'hidden' }}
                    />
                  }
                  onClick={() => setMessageTextTone(tone as MessageTextTone)}
                >
                  <Text size="B300">
                    {tone === 'soft' ? 'Soft' : tone === 'balanced' ? 'Balanced' : 'Bright'}
                  </Text>
                </Button>
              ))}
            </Box>
          }
        />
      </SequenceCard>

      <FontSettings />

      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <ThreadDisplaySetting />
      </SequenceCard>

      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title="Page zoom"
          description="Scale the whole interface, from 75% to 200%. Press Enter to apply."
          after={<PageZoomInput />}
        />
      </SequenceCard>
    </SettingsSection>
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
                    className={SettingsQuietControl}
                    size="300"
                    variant="Surface"
                    radii="300"
                    aria-pressed={messageLayout === item.layout}
                    after={
                      messageLayout === item.layout ? (
                        <Icon size="100" src={Icons.Check} />
                      ) : undefined
                    }
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
                    className={SettingsQuietControl}
                    size="300"
                    variant="Surface"
                    radii="300"
                    aria-pressed={messageSpacing === item.spacing}
                    after={
                      messageSpacing === item.spacing ? (
                        <Icon size="100" src={Icons.Check} />
                      ) : undefined
                    }
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

function MessageDisplay() {
  const [legacyUsernameColor, setLegacyUsernameColor] = useSetting(
    settingsAtom,
    'legacyUsernameColor'
  );

  return (
    <SettingsSection title="Messages">
      {!isNativeMatrixSession() && (
        <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
          <SettingTile title="Message layout" after={<SelectMessageLayout />} />
        </SequenceCard>
      )}
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile title="Message spacing" after={<SelectMessageSpacing />} />
      </SequenceCard>
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title="Legacy username color"
          description="Color sender names with the legacy per-user palette instead of the theme accent."
          after={
            <Switch
              variant="Primary"
              value={legacyUsernameColor}
              onChange={setLegacyUsernameColor}
            />
          }
        />
      </SequenceCard>
    </SettingsSection>
  );
}

const TILING_MODE_OPTIONS: Array<{ value: TilingModeSetting; label: string }> = [
  { value: 'auto', label: 'Auto' },
  { value: 'on', label: 'On' },
  { value: 'off', label: 'Off' },
];

function TilingModeTile() {
  const [tilingMode, setTilingMode] = useSetting(desktopPlatformSettingsAtom, 'tilingMode');
  const detected = getDesktopWindowEnvironment() === 'tiling';
  let status = '';
  if (tilingMode === 'auto') {
    status = detected
      ? ' A tiling window manager was detected, so it is on.'
      : ' No tiling window manager was detected, so it is off.';
  }

  return (
    <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
      <SettingTile
        title="Tiling mode"
        description={`For tiling window managers such as Hyprland or Sway: Synara leaves the window frame to them and draws no title bar of its own. Ctrl+Q still quits. Auto turns it on when Synara starts under one.${status}`}
        after={
          <SegmentedChoice
            label="Tiling mode"
            value={tilingMode}
            options={TILING_MODE_OPTIONS}
            onChange={setTilingMode}
          />
        }
      />
    </SequenceCard>
  );
}

/**
 * GPU drawing for the Linux web view. The shell reads the choice when it
 * starts, so a change waits for the next start. Shown only where the shell
 * says it applies (Linux under Wayland).
 */
function HardwareAccelerationTile() {
  const [status, setStatus] = useState(DESKTOP_RENDERING_UNAVAILABLE);
  const [saveFailed, setSaveFailed] = useState(false);

  useEffect(() => {
    let cancelled = false;
    void getDesktopRenderingStatus().then((next) => {
      if (!cancelled) setStatus(next);
    });
    return () => {
      cancelled = true;
    };
  }, []);

  if (!status.available) return null;
  const view = hardwareAccelerationView(status);

  const handleChange = (enabled: boolean) => {
    const previous = status;
    setSaveFailed(false);
    setStatus({ ...status, saved: enabled ? 'gpu' : 'safe' });
    void setDesktopRenderingMode(enabled ? 'gpu' : 'safe').then((next) => {
      setStatus(next ?? previous);
      setSaveFailed(!next);
    });
  };

  let note: string | undefined;
  if (view.locked) {
    note = `The SYNARA_RENDERING environment variable is deciding this (${
      view.enabled ? 'on' : 'off'
    }). Remove it to use this switch.`;
  } else if (saveFailed) {
    note = 'Synara could not save this setting.';
  } else if (view.restartNeeded) {
    note = 'Restart Synara to apply.';
  }

  return (
    <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
      <SettingTile
        title="Hardware acceleration"
        description="Draw the interface with your graphics card. Turn this off if the window flickers, goes blank or the app closes on start. Applies after you restart Synara."
        after={
          <Switch
            variant="Primary"
            value={view.enabled}
            onChange={handleChange}
            disabled={view.locked}
          />
        }
      >
        {note && (
          <Text size="T200" role="status">
            {note}
          </Text>
        )}
      </SettingTile>
    </SequenceCard>
  );
}

/** The window settings exist only in the borderless Linux shell. */
function WindowSettings() {
  if (!isLinuxDesktopShell()) return null;
  return (
    <SettingsSection title="Window">
      <TilingModeTile />
      <HardwareAccelerationTile />
    </SettingsSection>
  );
}

type AppearancePageProps = {
  requestClose: () => void;
};
export function AppearancePage({ requestClose }: AppearancePageProps) {
  return (
    <SettingsPage
      title="Appearance"
      description="These settings apply to this device only."
      requestClose={requestClose}
    >
      <Appearance />
      <TextAndZoom />
      <MessageDisplay />
      <WindowSettings />
    </SettingsPage>
  );
}
