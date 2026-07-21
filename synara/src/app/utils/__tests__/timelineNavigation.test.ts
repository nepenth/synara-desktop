import assert from 'node:assert/strict';
import test from 'node:test';
import {
  TimelineNavigationController,
  type TimelineNavigationFailure,
} from '../timelineNavigation';

type FakeTimer = {
  callback: () => void;
  timeoutMs: number;
  cancelled: boolean;
};

const createHarness = () => {
  const timers: FakeTimer[] = [];
  const timeouts: TimelineNavigationFailure<string>[] = [];
  const controller = new TimelineNavigationController<string>({
    routeKey: '!alpha:example.org\u0000$focus',
    loadingTimeoutMs: 15_000,
    settlingTimeoutMs: 5_750,
    onTimeout: (failure) => timeouts.push(failure),
    scheduler: {
      schedule: (callback, timeoutMs) => {
        const timer = { callback, timeoutMs, cancelled: false };
        timers.push(timer);
        return timer as unknown as ReturnType<typeof globalThis.setTimeout>;
      },
      cancel: (handle) => {
        (handle as unknown as FakeTimer).cancelled = true;
      },
    },
  });
  const fire = (timer: FakeTimer) => {
    if (!timer.cancelled) timer.callback();
  };
  return { controller, fire, timers, timeouts };
};

test('stale jump results cannot replace a newer navigation intent', () => {
  const { controller } = createHarness();
  const first = controller.beginJump('focused-window', '$focus');
  assert.equal(first, 1);
  assert.equal(controller.cancelForUser()?.previousTimeline, 'focused-window');

  const second = controller.beginJump('newer-window');
  assert.equal(second, 3);
  assert.equal(controller.resolveJump(first!, '$stale-tail'), false);
  assert.equal(controller.authoritativeTailEventId, undefined);
  assert.equal(controller.resolveJump(second!, '$current-tail'), true);
  assert.equal(controller.phase, 'settling');
  assert.equal(controller.authoritativeTailEventId, '$current-tail');
});

test('room and route changes cancel requests while intentional permalink clearing does not', () => {
  const { controller } = createHarness();
  const requestId = controller.beginJump('focused-window', '$focus')!;
  assert.equal(controller.resolveJump(requestId, '$latest'), true);

  const completion = controller.completeSettlement(true, '!alpha:example.org\u0000');
  assert.deepEqual(completion, { accepted: true, focusedEventId: '$focus' });
  assert.equal(controller.handleRouteChange('!alpha:example.org\u0000'), false);
  assert.equal(controller.authoritativeTailEventId, '$latest');

  const refresh = controller.beginLiveTailRefresh();
  assert.equal(controller.handleRouteChange('!beta:example.org\u0000'), true);
  assert.equal(controller.phase, 'idle');
  assert.equal(controller.authoritativeTailEventId, undefined);
  assert.equal(controller.applyLiveTailRefresh(refresh, '$stale-refresh'), false);
});

test('a route switch invalidates an unresolved jump result without restoring the old room', () => {
  const { controller } = createHarness();
  const requestId = controller.beginJump('alpha-window', '$focus')!;

  assert.equal(controller.handleRouteChange('!beta:example.org\u0000'), true);
  assert.equal(controller.phase, 'idle');
  assert.equal(controller.resolveJump(requestId, '$late-alpha-tail'), false);
  assert.equal(controller.authoritativeTailEventId, undefined);
});

test('a jump invalidates live-tail refreshes and refresh generations reject stale results', () => {
  const { controller } = createHarness();
  const oldRefresh = controller.beginLiveTailRefresh();
  controller.beginJump('old-window');
  assert.equal(controller.applyLiveTailRefresh(oldRefresh, '$old-refresh'), false);

  controller.cancelForUser();
  const firstRefresh = controller.beginLiveTailRefresh();
  const secondRefresh = controller.beginLiveTailRefresh();
  assert.equal(controller.applyLiveTailRefresh(firstRefresh, '$first'), false);
  assert.equal(controller.applyLiveTailRefresh(secondRefresh, '$second'), true);
  assert.equal(controller.authoritativeTailEventId, '$second');
});

test('detached latest tails own the live boundary and suppress forward pagination', () => {
  const { controller } = createHarness();
  const requestId = controller.beginJump('history-window')!;
  controller.resolveJump(requestId, '$detached-latest');

  assert.deepEqual(controller.getBounds('$detached-latest', false, true), {
    authoritativeLatestWindow: true,
    canPaginateForward: false,
    loadedAtEnd: true,
  });
  assert.equal(controller.getPersistedLiveTailEventId('$older-live-tail'), '$detached-latest');

  controller.reattachLiveTimeline();
  assert.equal(controller.authoritativeTailEventId, undefined);
  assert.deepEqual(controller.getBounds('$detached-latest', true, false), {
    authoritativeLatestWindow: false,
    canPaginateForward: false,
    loadedAtEnd: true,
  });
});

test('loading timeout rolls back and rejects its eventual asynchronous result', () => {
  const { controller, fire, timers, timeouts } = createHarness();
  const requestId = controller.beginJump('before-jump', '$focus')!;
  assert.equal(timers[0].timeoutMs, 15_000);

  fire(timers[0]);
  assert.deepEqual(timeouts, [
    {
      reason: 'loading-timeout',
      previousTimeline: 'before-jump',
    },
  ]);
  assert.equal(controller.phase, 'error');
  assert.equal(controller.resolveJump(requestId, '$too-late'), false);
});

test('settling replaces the loading deadline and supports cancel or confirmed completion', () => {
  const firstHarness = createHarness();
  const requestId = firstHarness.controller.beginJump('before-jump')!;
  assert.equal(firstHarness.controller.resolveJump(requestId, '$latest'), true);
  assert.equal(firstHarness.timers[0].cancelled, true);
  assert.equal(firstHarness.timers[1].timeoutMs, 5_750);
  assert.deepEqual(firstHarness.controller.cancelForUser(), {
    reason: 'user-cancelled',
    previousTimeline: 'before-jump',
  });
  assert.equal(firstHarness.timers[1].cancelled, true);

  const secondHarness = createHarness();
  const secondRequest = secondHarness.controller.beginJump('before-jump')!;
  secondHarness.controller.resolveJump(secondRequest, '$latest');
  secondHarness.fire(secondHarness.timers[1]);
  assert.deepEqual(secondHarness.timeouts, [
    {
      reason: 'settling-timeout',
      previousTimeline: 'before-jump',
    },
  ]);
  assert.equal(secondHarness.controller.phase, 'error');

  const thirdHarness = createHarness();
  const thirdRequest = thirdHarness.controller.beginJump('before-jump')!;
  thirdHarness.controller.resolveJump(thirdRequest, '$latest');
  assert.deepEqual(thirdHarness.controller.completeSettlement(false), {
    accepted: true,
    failure: {
      reason: 'settling-unconfirmed',
      previousTimeline: 'before-jump',
    },
  });
  assert.equal(thirdHarness.controller.phase, 'error');

  const fourthHarness = createHarness();
  const fourthRequest = fourthHarness.controller.beginJump('before-jump')!;
  fourthHarness.controller.resolveJump(fourthRequest, '$latest');
  assert.deepEqual(fourthHarness.controller.completeSettlement(true), {
    accepted: true,
    focusedEventId: undefined,
  });
  assert.equal(fourthHarness.controller.phase, 'idle');
  assert.equal(fourthHarness.controller.authoritativeTailEventId, '$latest');
});

test('an exception after a jump resolves still rolls back the settling window', () => {
  const { controller } = createHarness();
  const requestId = controller.beginJump('before-jump')!;
  controller.resolveJump(requestId, '$latest');
  assert.deepEqual(controller.rejectJump(requestId), {
    reason: 'request-error',
    previousTimeline: 'before-jump',
  });
  assert.equal(controller.phase, 'error');
});

test('live reattachment invalidates an in-flight refresh before clearing detached ownership', () => {
  const { controller } = createHarness();
  const refresh = controller.beginLiveTailRefresh();
  controller.reattachLiveTimeline();

  assert.equal(controller.applyLiveTailRefresh(refresh, '$detached'), false);
  assert.equal(controller.authoritativeTailEventId, undefined);
});
