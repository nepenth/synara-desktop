import { atom, useAtomValue, useSetAtom, type WritableAtom } from 'jotai';
import { selectAtom } from 'jotai/utils';
import { useMemo } from 'react';

type SettingsAtom<T extends object> = WritableAtom<T, [T], undefined>;
export type SettingSetter<T extends object, K extends keyof T> = T[K] | ((s: T[K]) => T[K]);

export const useSetSetting = <T extends object, K extends keyof T>(
  settingsAtom: SettingsAtom<T>,
  key: K
) => {
  const setterAtom = useMemo(
    () =>
      atom<null, [SettingSetter<T, K>], undefined>(null, (get, set, value) => {
        const current = get(settingsAtom);
        const next = { ...current };
        next[key] =
          typeof value === 'function' ? (value as (setting: T[K]) => T[K])(current[key]) : value;
        set(settingsAtom, next);
      }),
    [settingsAtom, key]
  );

  return useSetAtom(setterAtom);
};

export const useSetting = <T extends object, K extends keyof T>(
  settingsAtom: SettingsAtom<T>,
  key: K
): [T[K], ReturnType<typeof useSetSetting<T, K>>] => {
  const selector = useMemo(() => (s: T) => s[key], [key]);
  const setting = useAtomValue(selectAtom(settingsAtom, selector));

  const setter = useSetSetting(settingsAtom, key);
  return [setting, setter];
};
