import { useNativeNavigationScope } from '../../../state/hooks/navigationUnread';

export const useHomeRooms = () => useNativeNavigationScope('home').roomIds;
