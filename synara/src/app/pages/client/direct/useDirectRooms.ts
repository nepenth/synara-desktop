import { useNativeNavigationScope } from '../../../state/hooks/navigationUnread';

export const useDirectRooms = () => useNativeNavigationScope('direct').roomIds;
