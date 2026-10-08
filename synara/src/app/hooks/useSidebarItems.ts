import { Dispatch, SetStateAction, useEffect, useState } from 'react';
import { AccountDataEvent } from '../../types/matrix/accountData';
import { isSpace } from '../utils/room';
import { Membership } from '../../types/matrix/room';
import { getNativeRoom } from '../native/nativeSession';
import {
  getCachedAccountData,
  setNativeAccountData,
  useNativeAccountData,
} from '../native/nativeAccountData';

export type ISidebarFolder = {
  name?: string;
  id: string;
  content: string[];
};
export type TSidebarItem = string | ISidebarFolder;
export type SidebarItems = Array<TSidebarItem>;

export type InSynaraSpacesContent = {
  shortcut?: string[];
  sidebar?: SidebarItems;
};

export const parseSidebar = (orphanSpaces: string[], content?: InSynaraSpacesContent) => {
  const sidebar = content?.sidebar ?? content?.shortcut ?? [];
  const orphans = new Set(orphanSpaces);

  const items: SidebarItems = [];

  const safeToAdd = (spaceId: string): boolean => {
    if (typeof spaceId !== 'string') return false;
    const space = getNativeRoom(spaceId);
    if (space?.getMyMembership() !== Membership.Join) return false;
    return isSpace(space);
  };

  sidebar.forEach((item) => {
    if (typeof item === 'string') {
      if (safeToAdd(item) && !items.includes(item)) {
        orphans.delete(item);
        items.push(item);
      }
      return;
    }
    if (
      typeof item === 'object' &&
      typeof item.id === 'string' &&
      Array.isArray(item.content) &&
      !items.find((i) => (typeof i === 'string' ? false : i.id === item.id))
    ) {
      const safeContent = item.content.filter(safeToAdd);
      safeContent.forEach((i) => orphans.delete(i));
      items.push({
        ...item,
        content: Array.from(new Set(safeContent)),
      });
    }
  });

  orphans.forEach((spaceId) => items.push(spaceId));
  return items;
};

export const useSidebarItems = (
  orphanSpaces: string[]
): [SidebarItems, Dispatch<SetStateAction<SidebarItems>>] => {
  const content = useNativeAccountData(AccountDataEvent.SynaraSpaces) as
    InSynaraSpacesContent | null | undefined;
  const [sidebarItems, setSidebarItems] = useState(() =>
    parseSidebar(orphanSpaces, content ?? undefined)
  );

  useEffect(() => {
    setSidebarItems(parseSidebar(orphanSpaces, content ?? undefined));
  }, [orphanSpaces, content]);

  return [sidebarItems, setSidebarItems];
};

export const sidebarItemWithout = (items: SidebarItems, roomId: string) => {
  const newItems: SidebarItems = items
    .map((item) => {
      if (typeof item === 'string') {
        if (item === roomId) return null;
        return item;
      }
      if (item.content.includes(roomId)) {
        const newContent = item.content.filter((id) => id !== roomId);
        if (newContent.length === 0) return null;
        return {
          ...item,
          content: newContent,
        };
      }
      return item;
    })
    .filter((item) => item !== null) as SidebarItems;

  return newItems;
};

export const makeSynaraSpacesContent = (items: SidebarItems): InSynaraSpacesContent => {
  const currentInSpaces =
    (getCachedAccountData(AccountDataEvent.SynaraSpaces) as InSynaraSpacesContent | null) ?? {};

  const newSpacesContent: InSynaraSpacesContent = {
    ...currentInSpaces,
    sidebar: items,
  };

  return newSpacesContent;
};

/**
 * Persist the sidebar layout as `in.synara.spaces` account data. The cache
 * updates at once; a rejected write is reverted by the next Core refresh.
 */
export const saveSynaraSpacesContent = (content: InSynaraSpacesContent): Promise<void> =>
  setNativeAccountData(AccountDataEvent.SynaraSpaces, content as Record<string, unknown>);
