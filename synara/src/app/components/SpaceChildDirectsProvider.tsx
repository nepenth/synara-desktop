import { ReactNode } from 'react';
import { RoomToParents } from '../../types/matrix/room';
import { allRoomsAtom } from '../state/room-list/roomList';
import { useChildDirectScopeFactory, useSpaceChildren } from '../state/hooks/roomList';

type SpaceChildDirectsProviderProps = {
  spaceId: string;
  mDirects: Set<string>;
  roomToParents: RoomToParents;
  children: (rooms: string[]) => ReactNode;
};
export function SpaceChildDirectsProvider({
  spaceId,
  roomToParents,
  mDirects,
  children,
}: SpaceChildDirectsProviderProps) {
  const childDirects = useSpaceChildren(
    allRoomsAtom,
    spaceId,
    useChildDirectScopeFactory(mDirects, roomToParents)
  );

  return children(childDirects);
}
