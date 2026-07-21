const MAX_RENDERED_ROWS = 120;
const USER_SCROLL_IDLE_MS = 150;
const BOTTOM_TOLERANCE_PX = 2;

const app = document.querySelector('#app');
const scroller = document.querySelector('#timeline');
const rowsElement = document.querySelector('#rows');
const roomLabel = document.querySelector('#room-label');
const jumpLatestButton = document.querySelector('#jump-latest');

const nextFrame = () => new Promise((resolve) => requestAnimationFrame(resolve));
const delay = (durationMs) => new Promise((resolve) => window.setTimeout(resolve, durationMs));

const state = {
  rooms: new Map(),
  roomId: undefined,
  rangeStart: 0,
  rangeEnd: 0,
  generation: 0,
  phase: 'idle',
  lineHeight: 18,
  userScrolling: false,
  userScrollTimer: undefined,
  queuedMutations: [],
  flushing: Promise.resolve(),
  scrollWrites: [],
  staleOperations: 0,
};

const getRoomEvents = () => state.rooms.get(state.roomId)?.events ?? [];

const setPhase = (phase) => {
  state.phase = phase;
  app.dataset.phase = phase;
  jumpLatestButton.textContent = phase === 'loadingLatest' ? 'Loading latest…' : 'Jump to latest';
};

const updateJumpLatestVisibility = () => {
  const detachedFromLive = state.rangeEnd < getRoomEvents().length;
  const bottomGap = scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight;
  const confirmed = state.phase === 'bottomConfirmed' && bottomGap <= BOTTOM_TOLERANCE_PX;
  jumpLatestButton.hidden = !detachedFromLive && confirmed;
};

const makeBody = (event) =>
  Array.from({ length: event.lines }, (_, lineIndex) => `${event.id} line ${lineIndex + 1}`).join(
    '\n'
  );

const render = () => {
  const events = getRoomEvents().slice(state.rangeStart, state.rangeEnd);
  const fragment = document.createDocumentFragment();
  events.forEach((event) => {
    const row = document.createElement('article');
    row.className = 'event-row';
    row.dataset.eventId = event.id;
    row.dataset.lines = String(event.lines);

    const eventId = document.createElement('div');
    eventId.className = 'event-id';
    eventId.textContent = event.id;
    row.append(eventId);

    const body = document.createElement('p');
    body.className = 'event-body';
    body.style.lineHeight = `${state.lineHeight}px`;
    body.textContent = makeBody(event);
    row.append(body);

    if (event.extraHeight > 0) {
      const extra = document.createElement('div');
      extra.className = 'event-extra';
      extra.dataset.kind = event.extraKind;
      extra.style.height = `${event.extraHeight}px`;
      row.append(extra);
    }
    fragment.append(row);
  });
  rowsElement.replaceChildren(fragment);
  updateJumpLatestVisibility();
};

const getRow = (eventId) =>
  Array.from(rowsElement.querySelectorAll('.event-row')).find(
    (element) => element.dataset.eventId === eventId
  );

const captureAnchor = () => {
  const viewportTop = scroller.getBoundingClientRect().top;
  const rows = Array.from(rowsElement.querySelectorAll('.event-row'));
  const anchorRow =
    rows.find((row) => row.getBoundingClientRect().bottom > viewportTop) ?? rows.at(-1);
  if (!anchorRow) return undefined;
  return {
    eventId: anchorRow.dataset.eventId,
    offsetPx: anchorRow.getBoundingClientRect().top - viewportTop,
  };
};

const writeScrollBy = (deltaPx, reason) => {
  if (Math.abs(deltaPx) < 0.01) return;
  state.scrollWrites.push({
    activeUserScroll: state.userScrolling,
    deltaPx,
    reason,
    timestamp: performance.now(),
  });
  scroller.scrollBy({ top: deltaPx, behavior: 'instant' });
};

const writeScrollTop = (scrollTop, reason) => {
  const deltaPx = scrollTop - scroller.scrollTop;
  state.scrollWrites.push({
    activeUserScroll: state.userScrolling,
    deltaPx,
    reason,
    timestamp: performance.now(),
  });
  scroller.scrollTo({ top: scrollTop, behavior: 'instant' });
};

const restoreAnchor = async (anchor, reason) => {
  if (!anchor) return;
  for (let frame = 0; frame < 3; frame += 1) {
    await nextFrame();
    const row = getRow(anchor.eventId);
    if (!row) return;
    const currentOffset = row.getBoundingClientRect().top - scroller.getBoundingClientRect().top;
    writeScrollBy(currentOffset - anchor.offsetPx, `${reason}:frame-${frame + 1}`);
  }
};

const applyStructuralMutation = async (mutation, reason) => {
  const anchor = captureAnchor();
  mutation();
  render();
  await restoreAnchor(anchor, reason);
};

const flushQueuedMutations = async () => {
  if (state.userScrolling || state.queuedMutations.length === 0) return;
  const queued = state.queuedMutations.splice(0);
  const anchor = captureAnchor();
  queued.forEach(({ mutation }) => mutation());
  render();
  await restoreAnchor(anchor, `queued-${queued.map(({ reason }) => reason).join('+')}`);
};

const queueOrApply = async (mutation, reason) => {
  if (state.userScrolling) {
    state.queuedMutations.push({ mutation, reason });
    return { queued: true };
  }
  await applyStructuralMutation(mutation, reason);
  return { queued: false };
};

const setUserScrolling = (active) => {
  if (state.userScrollTimer !== undefined) {
    window.clearTimeout(state.userScrollTimer);
    state.userScrollTimer = undefined;
  }
  state.userScrolling = active;
  if (!active) {
    state.flushing = state.flushing.then(flushQueuedMutations);
  }
};

scroller.addEventListener(
  'wheel',
  () => {
    setUserScrolling(true);
    state.userScrollTimer = window.setTimeout(() => setUserScrolling(false), USER_SCROLL_IDLE_MS);
  },
  { passive: true }
);

const mutateEvent = (eventId, update) => {
  const event = getRoomEvents().find((candidate) => candidate.id === eventId);
  if (!event) throw new Error(`Unknown event: ${eventId}`);
  update(event);
};

const harness = {
  constants: {
    bottomTolerancePx: BOTTOM_TOLERANCE_PX,
    maxRenderedRows: MAX_RENDERED_ROWS,
    userScrollIdleMs: USER_SCROLL_IDLE_MS,
  },

  seedRoom(roomId, count, options = {}) {
    const { maxLines = 24 } = options;
    const events = Array.from({ length: count }, (_, index) => {
      let lines = 1 + ((index * 37) % maxLines);
      if (maxLines >= 200 && index === 0) lines = 1;
      if (maxLines >= 200 && index === 1) lines = 200;
      return {
        id: `${roomId}-event-${index}`,
        lines,
        extraHeight: 0,
        extraKind: undefined,
      };
    });
    state.rooms.set(roomId, { events });
    return events.map(({ id, lines }) => ({ id, lines }));
  },

  openRoom(roomId, options = {}) {
    const events = state.rooms.get(roomId)?.events;
    if (!events) throw new Error(`Unknown room: ${roomId}`);
    state.generation += 1;
    setUserScrolling(false);
    state.queuedMutations = [];
    state.roomId = roomId;
    roomLabel.textContent = roomId;
    const requestedEnd = options.end ?? events.length;
    state.rangeEnd = Math.min(events.length, Math.max(0, requestedEnd));
    const requestedStart = options.start ?? Math.max(0, state.rangeEnd - MAX_RENDERED_ROWS);
    state.rangeStart = Math.max(0, Math.min(requestedStart, state.rangeEnd));
    if (state.rangeEnd - state.rangeStart > MAX_RENDERED_ROWS) {
      state.rangeStart = state.rangeEnd - MAX_RENDERED_ROWS;
    }
    setPhase(state.rangeEnd === events.length ? 'idle' : 'focused');
    render();
    writeScrollTop(options.atBottom ? scroller.scrollHeight : 0, 'open-room');
    updateJumpLatestVisibility();
    return this.getState();
  },

  async scrollEventToOffset(eventId, offsetPx = 20) {
    const row = getRow(eventId);
    if (!row) throw new Error(`Event is not rendered: ${eventId}`);
    const deltaPx =
      row.getBoundingClientRect().top - scroller.getBoundingClientRect().top - offsetPx;
    writeScrollBy(deltaPx, 'scroll-event-to-offset');
    await nextFrame();
    return captureAnchor();
  },

  captureAnchor,

  getAnchorDrift(anchor) {
    const row = getRow(anchor.eventId);
    if (!row) return Number.POSITIVE_INFINITY;
    const offsetPx = row.getBoundingClientRect().top - scroller.getBoundingClientRect().top;
    return Math.abs(offsetPx - anchor.offsetPx);
  },

  async prepend(count) {
    return queueOrApply(() => {
      state.rangeStart = Math.max(0, state.rangeStart - count);
      if (state.rangeEnd - state.rangeStart > MAX_RENDERED_ROWS) {
        state.rangeStart = state.rangeEnd - MAX_RENDERED_ROWS;
      }
    }, 'prepend');
  },

  async expandImage(eventId, heightPx = 240) {
    return queueOrApply(
      () =>
        mutateEvent(eventId, (event) => {
          event.extraHeight = heightPx;
          event.extraKind = 'image';
        }),
      'late-image'
    );
  },

  async decryptEvent(eventId, lineCount) {
    return queueOrApply(
      () => mutateEvent(eventId, (event) => (event.lines = lineCount)),
      'late-decryption'
    );
  },

  async expandReply(eventId, lineCount = 8) {
    return queueOrApply(
      () =>
        mutateEvent(eventId, (event) => {
          event.extraHeight = lineCount * state.lineHeight;
          event.extraKind = 'reply';
        }),
      'reply-expansion'
    );
  },

  async loadFontMetrics(lineHeightPx) {
    return queueOrApply(() => (state.lineHeight = lineHeightPx), 'late-font');
  },

  beginUserScroll() {
    setUserScrolling(true);
  },

  endUserScroll() {
    setUserScrolling(false);
    return state.flushing;
  },

  async waitForIdle() {
    while (state.userScrolling) await delay(10);
    await state.flushing;
    await nextFrame();
  },

  async jumpLatest(options = {}) {
    const { delayMs = 30, fail = false } = options;
    const generation = state.generation;
    const roomId = state.roomId;
    setPhase('loadingLatest');
    jumpLatestButton.hidden = false;
    await delay(delayMs);
    if (generation !== state.generation || roomId !== state.roomId) {
      state.staleOperations += 1;
      return { ok: false, stale: true };
    }
    if (fail) {
      setPhase('latestError');
      jumpLatestButton.hidden = false;
      return { ok: false, stale: false };
    }
    await this.waitForIdle();
    if (generation !== state.generation || roomId !== state.roomId) {
      state.staleOperations += 1;
      return { ok: false, stale: true };
    }

    const events = getRoomEvents();
    state.rangeEnd = events.length;
    state.rangeStart = Math.max(0, state.rangeEnd - MAX_RENDERED_ROWS);
    setPhase('rebindingLive');
    render();
    writeScrollTop(scroller.scrollHeight, 'jump-latest-live-tail');
    setPhase('settlingLayout');
    await nextFrame();
    await nextFrame();
    const bottomGap = scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight;
    if (bottomGap > BOTTOM_TOLERANCE_PX) {
      setPhase('latestError');
      jumpLatestButton.hidden = false;
      return { ok: false, stale: false, bottomGap };
    }

    setPhase('bottomConfirmed');
    jumpLatestButton.hidden = true;
    return { ok: true, stale: false, bottomGap };
  },

  clearScrollWrites() {
    state.scrollWrites = [];
  },

  getRowMetrics() {
    const rows = Array.from(rowsElement.querySelectorAll('.event-row'));
    const lineCounts = rows.map((row) => Number.parseInt(row.dataset.lines, 10));
    const heights = rows.map((row) => row.getBoundingClientRect().height);
    return {
      count: rows.length,
      maxHeight: Math.max(...heights),
      maxLines: Math.max(...lineCounts),
      minHeight: Math.min(...heights),
      minLines: Math.min(...lineCounts),
    };
  },

  getState() {
    const bottomGap = scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight;
    return {
      bottomGap,
      generation: state.generation,
      jumpLatestHidden: jumpLatestButton.hidden,
      phase: state.phase,
      queuedMutationCount: state.queuedMutations.length,
      rangeEnd: state.rangeEnd,
      rangeStart: state.rangeStart,
      renderedRowCount: rowsElement.children.length,
      roomId: state.roomId,
      scrollWrites: state.scrollWrites.map((write) => ({ ...write })),
      staleOperations: state.staleOperations,
      userScrolling: state.userScrolling,
    };
  },
};

jumpLatestButton.addEventListener('click', () => harness.jumpLatest());
window.timelineHarness = harness;
