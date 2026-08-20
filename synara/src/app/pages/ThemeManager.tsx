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
import { settingsAtom } from '../state/settings';
import { normalizeAccentColor } from '../utils/themeAccent';
import {
  deriveThemeSurfaceRamp,
  resolveThemeBaseColor,
  type ThemeSurfaceScale,
} from '../utils/themeBase';

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

const syncThemeBaseColor = (baseColor: string | undefined, themeKind: ThemeKind): string => {
  const ramp = deriveThemeSurfaceRamp(resolveThemeBaseColor(baseColor), themeKind);
  const target = document.body;

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
    const chrome = syncThemeBaseColor(undefined, systemThemeKind);
    syncDocumentThemeChrome(systemThemeKind, chrome);
  }, [systemThemeKind]);

  return null;
}

export function AuthRouteThemeManager({ children }: { children: ReactNode }) {
  const activeTheme = useActiveTheme();
  const [monochromeMode] = useSetting(settingsAtom, 'monochromeMode');
  const [customAccentColor] = useSetting(settingsAtom, 'customAccentColor');
  const [themeBaseColor] = useSetting(settingsAtom, 'themeBaseColor');

  useEffect(() => {
    document.body.className = '';
    document.body.classList.add(configClass, varsClass);

    document.body.classList.add(...activeTheme.classNames);
    const chrome = syncThemeBaseColor(themeBaseColor, activeTheme.kind);
    syncDocumentThemeChrome(activeTheme.kind, chrome);
    syncAccentColor(customAccentColor, activeTheme.kind);

    if (monochromeMode) {
      document.body.style.filter = 'grayscale(1)';
    } else {
      document.body.style.filter = '';
    }
  }, [activeTheme, monochromeMode, customAccentColor, themeBaseColor]);

  return <ThemeContextProvider value={activeTheme}>{children}</ThemeContextProvider>;
}
