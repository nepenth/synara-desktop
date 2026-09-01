import React, { ReactNode, useEffect } from 'react';
import { color, configClass, varsClass } from 'folds';
import {
  DarkTheme,
  LightTheme,
  ThemeContextProvider,
  ThemeKind,
  useActiveTheme,
  useSystemThemeKind,
} from '../hooks/useTheme';
import { useSetting } from '../state/hooks/settings';
import { getSharedSettings, settingsAtom } from '../state/settings';
import { normalizeAccentColor } from '../utils/themeAccent';
import { messageTextForeground } from '../utils/messageTextTone';
import {
  deriveThemeSurfaceRamp,
  deriveThemeRichTextRoles,
  resolveThemeBaseColor,
  shouldApplyDerivedThemeRamp,
  type ThemeContentRoles,
  type ThemeRichTextRoles,
  type ThemeSurfaceScale,
} from '../utils/themeBase';

const LEGACY_RICH_TEXT_SURFACES: Record<
  string,
  { kind: ThemeKind; readingSurface: string; readingSurfaceHover: string }
> = {
  'silver-theme': {
    kind: ThemeKind.Light,
    readingSurface: '#DEDEDE',
    readingSurfaceHover: '#D3D3D3',
  },
  'butter-theme': {
    kind: ThemeKind.Dark,
    readingSurface: '#33322C',
    readingSurfaceHover: '#403F38',
  },
};

const RICH_TEXT_ROLE_VARIABLES: Record<keyof ThemeRichTextRoles, string> = {
  readingSurface: '--synara-rich-text-reading-surface',
  readingSurfaceHover: '--synara-rich-text-reading-surface-hover',
  inlineCodeBackground: '--synara-rich-text-inline-code-background',
  inlineCodeBorder: '--synara-rich-text-inline-code-border',
  inlineCodeForeground: '--synara-rich-text-inline-code-foreground',
  codeBlockBackground: '--synara-rich-text-code-block-background',
  codeBlockBorder: '--synara-rich-text-code-block-border',
  spoilerBackground: '--synara-rich-text-spoiler-background',
  spoilerHover: '--synara-rich-text-spoiler-hover',
  spoilerBorder: '--synara-rich-text-spoiler-border',
  tableCanvas: '--synara-rich-text-table-canvas',
  tableHeader: '--synara-rich-text-table-header',
  tableOdd: '--synara-rich-text-table-odd',
  tableEven: '--synara-rich-text-table-even',
  tableHover: '--synara-rich-text-table-hover',
  contrastBorder: '--synara-rich-text-contrast-border',
  contrastForeground: '--synara-rich-text-contrast-foreground',
};

const CONTENT_ROLE_VARIABLES: Record<keyof ThemeContentRoles, string> = {
  heading: '--synara-content-heading',
  primary: '--synara-content-primary',
  secondary: '--synara-content-secondary',
  tertiary: '--synara-content-tertiary',
  separator: '--synara-content-separator',
  selectedSurface: '--synara-selected-surface',
  tableCanvas: '--synara-table-canvas',
  tableHeader: '--synara-table-header',
  tableOdd: '--synara-table-odd',
  tableEven: '--synara-table-even',
  tableHover: '--synara-table-hover',
};

const syncContentRoles = (target: HTMLElement, roles: ThemeContentRoles) => {
  (Object.keys(CONTENT_ROLE_VARIABLES) as Array<keyof ThemeContentRoles>).forEach((role) => {
    target.style.setProperty(CONTENT_ROLE_VARIABLES[role], roles[role]);
  });
};

const clearContentRoles = (target: HTMLElement) => {
  Object.values(CONTENT_ROLE_VARIABLES).forEach((variable) => {
    target.style.removeProperty(variable);
  });
};

const syncRichTextRoles = (target: HTMLElement, roles: ThemeRichTextRoles) => {
  (Object.keys(RICH_TEXT_ROLE_VARIABLES) as Array<keyof ThemeRichTextRoles>).forEach((role) => {
    target.style.setProperty(RICH_TEXT_ROLE_VARIABLES[role], roles[role]);
  });
};

const clearRichTextRoles = (target: HTMLElement) => {
  Object.values(RICH_TEXT_ROLE_VARIABLES).forEach((variable) => {
    target.style.removeProperty(variable);
  });
};

const syncDocumentThemeChrome = (themeKind: ThemeKind, chromeColor: string) => {
  const themeColorMeta = document.querySelector<HTMLMetaElement>('meta[name="theme-color"]');

  document.documentElement.style.colorScheme = themeKind;
  document.documentElement.style.backgroundColor = chromeColor;
  document.body.style.colorScheme = themeKind;
  document.body.style.backgroundColor = chromeColor;
  themeColorMeta?.setAttribute('content', chromeColor);
};

const cssVarName = (value: string): string | undefined => value.match(/^var\((--[^)]+)\)$/)?.[1];

const setColorVar = (target: HTMLElement, token: string, value: string) => {
  const varName = cssVarName(token);
  if (varName) target.style.setProperty(varName, value);
};

const clearColorVar = (target: HTMLElement, token: string) => {
  const varName = cssVarName(token);
  if (varName) target.style.removeProperty(varName);
};

const applyContainerScale = (
  target: HTMLElement,
  tokens: ThemeSurfaceScale,
  scale: ThemeSurfaceScale
) => {
  setColorVar(target, tokens.Container, scale.Container);
  setColorVar(target, tokens.ContainerHover, scale.ContainerHover);
  setColorVar(target, tokens.ContainerActive, scale.ContainerActive);
  setColorVar(target, tokens.ContainerLine, scale.ContainerLine);
  setColorVar(target, tokens.OnContainer, scale.OnContainer);
};

const CONTAINER_TOKENS: ThemeSurfaceScale[] = [
  color.Background,
  color.Surface,
  color.SurfaceVariant,
  color.Secondary,
];

const clearContainerScale = (target: HTMLElement, tokens: ThemeSurfaceScale) => {
  clearColorVar(target, tokens.Container);
  clearColorVar(target, tokens.ContainerHover);
  clearColorVar(target, tokens.ContainerActive);
  clearColorVar(target, tokens.ContainerLine);
  clearColorVar(target, tokens.OnContainer);
};

const syncThemeBaseColor = (
  baseColor: string | undefined,
  themeKind: ThemeKind,
  themeId?: string
): string => {
  const target = document.body;
  const ramp = deriveThemeSurfaceRamp(resolveThemeBaseColor(baseColor), themeKind);
  if (!shouldApplyDerivedThemeRamp(themeId)) {
    clearContentRoles(target);
    const legacySurface = themeId ? LEGACY_RICH_TEXT_SURFACES[themeId] : undefined;
    if (legacySurface) {
      syncRichTextRoles(
        target,
        deriveThemeRichTextRoles(
          legacySurface.kind,
          legacySurface.readingSurface,
          legacySurface.readingSurfaceHover
        )
      );
    } else {
      clearRichTextRoles(target);
    }
    CONTAINER_TOKENS.forEach((tokens) => clearContainerScale(target, tokens));
    clearColorVar(target, color.Other.FocusRing);
    clearColorVar(target, color.Other.Shadow);
    clearColorVar(target, color.Other.Overlay);
    return themeKind === ThemeKind.Dark ? '#1e1f22' : '#ffffff';
  }

  syncContentRoles(target, ramp.content);
  syncRichTextRoles(target, ramp.richText);
  applyContainerScale(target, color.Background, ramp.background);
  applyContainerScale(target, color.Surface, ramp.surface);
  applyContainerScale(target, color.SurfaceVariant, ramp.surfaceVariant);
  applyContainerScale(target, color.Secondary, ramp.secondaryContainer);
  setColorVar(target, color.Other.FocusRing, ramp.focusRing);
  setColorVar(target, color.Other.Shadow, ramp.shadow);
  setColorVar(target, color.Other.Overlay, ramp.overlay);

  return ramp.chrome;
};

const syncAccentColor = (accentColor: string | undefined, themeKind: ThemeKind) => {
  const target = document.body;
  const normalized = normalizeAccentColor(accentColor);
  const primaryTokens = [
    color.Primary.Main,
    color.Primary.MainHover,
    color.Primary.MainActive,
    color.Primary.MainLine,
    color.Primary.Container,
    color.Primary.ContainerHover,
    color.Primary.ContainerActive,
    color.Primary.ContainerLine,
    color.Primary.OnContainer,
  ];

  if (!normalized) {
    primaryTokens.forEach((token) => clearColorVar(target, token));
    document.documentElement.style.removeProperty('--tc-link');
    return;
  }

  const dark = themeKind === ThemeKind.Dark;
  setColorVar(target, color.Primary.Main, normalized);
  setColorVar(target, color.Primary.MainHover, `color-mix(in srgb, ${normalized} 88%, black)`);
  setColorVar(target, color.Primary.MainActive, `color-mix(in srgb, ${normalized} 78%, black)`);
  setColorVar(target, color.Primary.MainLine, `color-mix(in srgb, ${normalized} 68%, black)`);
  setColorVar(
    target,
    color.Primary.Container,
    `color-mix(in srgb, ${normalized} ${dark ? '28%' : '22%'}, ${dark ? 'black' : 'white'})`
  );
  setColorVar(
    target,
    color.Primary.ContainerHover,
    `color-mix(in srgb, ${normalized} ${dark ? '34%' : '28%'}, ${dark ? 'black' : 'white'})`
  );
  setColorVar(
    target,
    color.Primary.ContainerActive,
    `color-mix(in srgb, ${normalized} ${dark ? '40%' : '34%'}, ${dark ? 'black' : 'white'})`
  );
  setColorVar(
    target,
    color.Primary.ContainerLine,
    `color-mix(in srgb, ${normalized} ${dark ? '48%' : '42%'}, ${dark ? 'black' : 'white'})`
  );
  setColorVar(target, color.Primary.OnContainer, dark ? '#f7fffb' : '#10251f');
  document.documentElement.style.setProperty('--tc-link', normalized);
};

export function UnAuthRouteThemeManager() {
  const systemThemeKind = useSystemThemeKind();

  useEffect(() => {
    document.body.className = '';
    document.body.classList.add(configClass, varsClass);
    if (systemThemeKind === ThemeKind.Dark) {
      document.body.classList.add(...DarkTheme.classNames);
    }
    if (systemThemeKind === ThemeKind.Light) {
      document.body.classList.add(...LightTheme.classNames);
    }
    const storedBase = getSharedSettings().themeBaseColor;
    const themeId = systemThemeKind === ThemeKind.Dark ? DarkTheme.id : LightTheme.id;
    const chrome = syncThemeBaseColor(storedBase, systemThemeKind, themeId);
    syncDocumentThemeChrome(systemThemeKind, chrome);
  }, [systemThemeKind]);

  return null;
}

export function AuthRouteThemeManager({ children }: { children: ReactNode }) {
  const activeTheme = useActiveTheme();
  const [monochromeMode] = useSetting(settingsAtom, 'monochromeMode');
  const [customAccentColor] = useSetting(settingsAtom, 'customAccentColor');
  const [themeBaseColor] = useSetting(settingsAtom, 'themeBaseColor');
  const [messageTextTone] = useSetting(settingsAtom, 'messageTextTone');

  useEffect(() => {
    document.body.className = '';
    document.body.classList.add(configClass, varsClass);

    document.body.classList.add(...activeTheme.classNames);
    const chrome = syncThemeBaseColor(themeBaseColor, activeTheme.kind, activeTheme.id);
    syncDocumentThemeChrome(activeTheme.kind, chrome);
    syncAccentColor(customAccentColor, activeTheme.kind);
    document.body.style.setProperty(
      '--synara-message-foreground',
      messageTextForeground(messageTextTone, activeTheme.kind === ThemeKind.Dark ? 'dark' : 'light')
    );

    if (monochromeMode) {
      document.body.style.filter = 'grayscale(1)';
    } else {
      document.body.style.filter = '';
    }
  }, [activeTheme, monochromeMode, customAccentColor, themeBaseColor, messageTextTone]);

  return <ThemeContextProvider value={activeTheme}>{children}</ThemeContextProvider>;
}
