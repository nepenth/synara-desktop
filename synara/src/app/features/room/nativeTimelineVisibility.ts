/** Observe painted geometry, including rooms that fit without ever scrolling. */
export const observeNativeTimelineBottom = (
  element: HTMLElement,
  onBottomChanged: (atBottom: boolean) => void
): (() => void) => {
  let frame = 0;
  let disposed = false;
  let needsResubscribe = false;
  const resize = new ResizeObserver(() => schedule());
  const measure = () => {
    frame = 0;
    if (disposed) return;
    if (needsResubscribe) {
      needsResubscribe = false;
      resize.disconnect();
      resize.observe(element);
      Array.from(element.children).forEach((child) => resize.observe(child));
    }
    onBottomChanged(
      element.clientHeight > 0 &&
        element.scrollHeight - element.scrollTop - element.clientHeight <= 8
    );
  };
  const schedule = (resubscribe = false) => {
    if (resubscribe) needsResubscribe = true;
    if (!disposed && frame === 0) frame = requestAnimationFrame(measure);
  };
  const onScroll = () => schedule();
  resize.observe(element);
  const mutations = new MutationObserver(() => schedule(true));
  mutations.observe(element, { childList: true });
  element.addEventListener('scroll', onScroll, { passive: true });
  schedule(true);
  return () => {
    disposed = true;
    cancelAnimationFrame(frame);
    resize.disconnect();
    mutations.disconnect();
    element.removeEventListener('scroll', onScroll);
  };
};
