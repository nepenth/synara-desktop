import { Dispatch, SetStateAction, useEffect, useState } from 'react';
import { AccountDataEvent } from '../../types/matrix/accountData';
import { getAccountData, isSpace } from '../utils/room';
import { Membership } from '../../types/matrix/room';

import { getNativeRoom } from '../native/nativeSession';
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
  const [sidebarItems, setSidebarItems] = useState(() => {
    const inSynaraSpacesContent = getAccountData(
      AccountDataEvent.SynaraSpaces
    )?.getContent<InSynaraSpacesContent>();
    return parseSidebar(orphanSpaces, inSynaraSpacesContent);
  });

  useEffect(() => {
    const inSynaraSpacesContent = getAccountData(
      AccountDataEvent.SynaraSpaces
    )?.getContent<InSynaraSpacesContent>();
    setSidebarItems(parseSidebar(orphanSpaces, inSynaraSpacesContent));
  }, [orphanSpaces]);

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
    getAccountData(AccountDataEvent.SynaraSpaces)?.getContent<InSynaraSpacesContent>() ?? {};

  const newSpacesContent: InSynaraSpacesContent = {
    ...currentInSpaces,
    sidebar: items,
  };

  return newSpacesContent;
};
