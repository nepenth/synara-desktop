/** Observe painted geometry, including rooms that fit without ever scrolling. */
export const observeNativeTimelineBottom = (
  element: HTMLElement,
  onBottomChanged: (atBottom: boolean) => void
): (() => void) => {
  let frame = 0;
  let disposed = false;
  const measure = () => {
    frame = 0;
    if (disposed) return;
    onBottomChanged(
      element.clientHeight > 0 &&
        element.scrollHeight - element.scrollTop - element.clientHeight <= 8
    );
  };
  const schedule = () => {
    if (!disposed && frame === 0) frame = requestAnimationFrame(measure);
  };
  const resize = new ResizeObserver(schedule);
  resize.observe(element);
  const observeContent = () => {
    resize.disconnect();
    resize.observe(element);
    Array.from(element.children).forEach((child) => resize.observe(child));
    schedule();
  };
  const mutations = new MutationObserver(observeContent);
  mutations.observe(element, { childList: true });
  element.addEventListener('scroll', schedule, { passive: true });
  observeContent();
  return () => {
    disposed = true;
    cancelAnimationFrame(frame);
    resize.disconnect();
    mutations.disconnect();
    element.removeEventListener('scroll', schedule);
  };
};
