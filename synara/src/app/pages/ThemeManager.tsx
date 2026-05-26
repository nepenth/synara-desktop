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

const THEME_CHROME_COLOR: Record<ThemeKind, string> = {
  [ThemeKind.Light]: '#ffffff',
  [ThemeKind.Dark]: '#1b1b1b',
};

const syncDocumentThemeChrome = (themeKind: ThemeKind) => {
  const themeColor = THEME_CHROME_COLOR[themeKind];
  const themeColorMeta = document.querySelector<HTMLMetaElement>('meta[name="theme-color"]');

  document.documentElement.style.colorScheme = themeKind;
  document.documentElement.style.backgroundColor = themeColor;
  document.body.style.colorScheme = themeKind;
  document.body.style.backgroundColor = themeColor;
  themeColorMeta?.setAttribute('content', themeColor);
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
    syncDocumentThemeChrome(systemThemeKind);
    if (systemThemeKind === ThemeKind.Dark) {
      document.body.classList.add(...DarkTheme.classNames);
    }
    if (systemThemeKind === ThemeKind.Light) {
      document.body.classList.add(...LightTheme.classNames);
    }
  }, [systemThemeKind]);

  return null;
}

export function AuthRouteThemeManager({ children }: { children: ReactNode }) {
  const activeTheme = useActiveTheme();
  const [monochromeMode] = useSetting(settingsAtom, 'monochromeMode');
  const [customAccentColor] = useSetting(settingsAtom, 'customAccentColor');

  useEffect(() => {
    document.body.className = '';
    document.body.classList.add(configClass, varsClass);

    document.body.classList.add(...activeTheme.classNames);
    syncDocumentThemeChrome(activeTheme.kind);
    syncAccentColor(customAccentColor, activeTheme.kind);

    if (monochromeMode) {
      document.body.style.filter = 'grayscale(1)';
    } else {
      document.body.style.filter = '';
    }
  }, [activeTheme, monochromeMode, customAccentColor]);

  return <ThemeContextProvider value={activeTheme}>{children}</ThemeContextProvider>;
}
