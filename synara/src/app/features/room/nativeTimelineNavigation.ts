/** Native viewport intent after a successful composer send; contains no Matrix write. */
const eventName = 'synara:room-message-sent';
export const requestRoomLatestAfterSend = (roomId: string): void => {
  window.dispatchEvent(new CustomEvent<string>(eventName, { detail: roomId }));
};
export const observeRoomLatestAfterSend = (roomId: string, navigate: () => void): (() => void) => {
  const listener = (event: Event) => {
    if (event instanceof CustomEvent && event.detail === roomId) navigate();
  };
  window.addEventListener(eventName, listener);
  return () => window.removeEventListener(eventName, listener);
};
