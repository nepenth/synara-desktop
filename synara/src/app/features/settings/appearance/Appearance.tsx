import React, {
  ChangeEventHandler,
  KeyboardEventHandler,
  MouseEventHandler,
  useState,
} from 'react';
import {
  as,
  Box,
  Button,
  Chip,
  config,
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
import { MessageLayout, MessageSpacing, settingsAtom } from '../../../state/settings';
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
import { SequenceCardStyle } from '../styles.css';
import { isNativeMatrixSession } from '../../verification/nativeVerification';
import { normalizeAccentColor, themeDefaultAccentColor } from '../../../utils/themeAccent';
import { MESSAGE_TEXT_TONES, type MessageTextTone } from '../../../utils/messageTextTone';
import {
  chromeColorsForRamp,
  DEFAULT_THEME_BASE_COLOR,
  deriveThemeSurfaceRamp,
  normalizeThemeBaseColor,
  THEME_BASE_PRESETS,
} from '../../../utils/themeBase';

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
      <Text size="L400">Theme</Text>
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
          description="Render the interface in greyscale, keeping only the accent color."
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
    </Box>
  );
}

function TextAndZoom() {
  const [messageTextTone, setMessageTextTone] = useSetting(settingsAtom, 'messageTextTone');

  return (
    <Box direction="Column" gap="100">
      <Text size="L400">Text</Text>
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title="Message Text"
          description="Adjust message prose brightness without changing navigation, metadata, or code colors."
          after={
            <Box gap="100" role="group" aria-label="Message text brightness">
              {MESSAGE_TEXT_TONES.map((tone) => (
                <Button
                  key={tone}
                  size="300"
                  radii="300"
                  variant={messageTextTone === tone ? 'Primary' : 'Secondary'}
                  fill={messageTextTone === tone ? 'Solid' : 'Soft'}
                  aria-pressed={messageTextTone === tone}
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

      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title="Page Zoom"
          description="Scale the whole interface, from 75% to 150%. Press Enter to apply."
          after={<PageZoomInput />}
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

function MessageDisplay() {
  const [legacyUsernameColor, setLegacyUsernameColor] = useSetting(
    settingsAtom,
    'legacyUsernameColor'
  );

  return (
    <Box direction="Column" gap="100">
      <Text size="L400">Messages</Text>
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title="Message Layout"
          description={
            isNativeMatrixSession()
              ? 'The native timeline uses a single Element-like layout. Compact and Bubble apply only to the retired JS timeline.'
              : undefined
          }
          after={
            isNativeMatrixSession() ? <Text size="T300">Modern</Text> : <SelectMessageLayout />
          }
        />
      </SequenceCard>
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile title="Message Spacing" after={<SelectMessageSpacing />} />
      </SequenceCard>
      <SequenceCard className={SequenceCardStyle} variant="SurfaceVariant" direction="Column">
        <SettingTile
          title="Legacy Username Color"
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
    </Box>
  );
}

type AppearancePageProps = {
  requestClose: () => void;
};
export function AppearancePage({ requestClose }: AppearancePageProps) {
  return (
    <Page>
      <PageHeader outlined={false}>
        <Box grow="Yes" gap="200">
          <Box grow="Yes" alignItems="Center" gap="200">
            <Text size="H3" truncate>
              Appearance
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
              <TextAndZoom />
              <MessageDisplay />
            </Box>
          </PageContent>
        </Scroll>
      </Box>
    </Page>
  );
}
