export type TimelineNavigationPhase = 'idle' | 'loading' | 'settling' | 'error';

export type TimelineNavigationFailureReason =
  | 'request-error'
  | 'user-cancelled'
  | 'loading-timeout'
  | 'settling-timeout'
  | 'settling-unconfirmed';

export type TimelineNavigationFailure<TTimeline> = {
  reason: TimelineNavigationFailureReason;
  previousTimeline?: TTimeline;
};

export type TimelineNavigationCompletion = {
  accepted: boolean;
  focusedEventId?: string;
};

export type TimelineNavigationBounds = {
  authoritativeLatestWindow: boolean;
  canPaginateForward: boolean;
  loadedAtEnd: boolean;
};

export type TimelineNavigationSnapshot = {
  phase: TimelineNavigationPhase;
  requestGeneration: number;
  liveTailRefreshGeneration: number;
  authoritativeTailEventId?: string;
};

type TimeoutHandle = ReturnType<typeof globalThis.setTimeout>;

type TimelineNavigationScheduler = {
  schedule: (callback: () => void, timeoutMs: number) => TimeoutHandle;
  cancel: (handle: TimeoutHandle) => void;
};

type TimelineNavigationControllerOptions<TTimeline> = {
  routeKey: string;
  loadingTimeoutMs: number;
  settlingTimeoutMs: number;
  onTimeout: (failure: TimelineNavigationFailure<TTimeline>) => void;
  scheduler?: TimelineNavigationScheduler;
};

const defaultScheduler: TimelineNavigationScheduler = {
  schedule: (callback, timeoutMs) => globalThis.setTimeout(callback, timeoutMs),
  cancel: (handle) => globalThis.clearTimeout(handle),
};

/**
 * Owns the asynchronous navigation lifecycle independently from the timeline view.
 *
 * The view remains responsible for fetching SDK timelines and positioning rows. This
 * controller decides whether an asynchronous result is still current, when a jump
 * must roll back, and whether a detached latest context is authoritative. Keeping
 * those decisions together prevents route, refresh, and timeout races from mutating
 * the viewport after a newer navigation intent.
 */
export class TimelineNavigationController<TTimeline> {
  private readonly loadingTimeoutMs: number;

  private readonly settlingTimeoutMs: number;

  private readonly onTimeout: (failure: TimelineNavigationFailure<TTimeline>) => void;

  private readonly scheduler: TimelineNavigationScheduler;

  private timeoutHandle?: TimeoutHandle;

  private routeKey: string;

  private phaseValue: TimelineNavigationPhase = 'idle';

  private requestGeneration = 0;

  private liveTailRefreshGeneration = 0;

  private previousTimeline?: TTimeline;

  private focusedEventId?: string;

  private authoritativeTailEventIdValue?: string;

  private expectedRouteKey?: string;

  public constructor(options: TimelineNavigationControllerOptions<TTimeline>) {
    this.routeKey = options.routeKey;
    this.loadingTimeoutMs = options.loadingTimeoutMs;
    this.settlingTimeoutMs = options.settlingTimeoutMs;
    this.onTimeout = options.onTimeout;
    this.scheduler = options.scheduler ?? defaultScheduler;
  }

  public get phase(): TimelineNavigationPhase {
    return this.phaseValue;
  }

  public get authoritativeTailEventId(): string | undefined {
    return this.authoritativeTailEventIdValue;
  }

  public get snapshot(): TimelineNavigationSnapshot {
    return {
      phase: this.phaseValue,
      requestGeneration: this.requestGeneration,
      liveTailRefreshGeneration: this.liveTailRefreshGeneration,
      authoritativeTailEventId: this.authoritativeTailEventIdValue,
    };
  }

  public beginJump(previousTimeline: TTimeline, focusedEventId?: string): number | undefined {
    if (this.phaseValue === 'loading' || this.phaseValue === 'settling') return undefined;

    this.invalidateLiveTailRefresh();
    this.requestGeneration += 1;
    this.previousTimeline = previousTimeline;
    this.focusedEventId = focusedEventId;
    this.expectedRouteKey = undefined;
    this.phaseValue = 'loading';
    this.schedulePhaseTimeout('loading', this.requestGeneration);
    return this.requestGeneration;
  }

  public resolveJump(requestId: number, authoritativeTailEventId: string): boolean {
    if (requestId !== this.requestGeneration || this.phaseValue !== 'loading') return false;

    this.authoritativeTailEventIdValue = authoritativeTailEventId;
    this.phaseValue = 'settling';
    this.schedulePhaseTimeout('settling', requestId);
    return true;
  }

  public rejectJump(
    requestId: number,
    reason: TimelineNavigationFailureReason = 'request-error'
  ): TimelineNavigationFailure<TTimeline> | undefined {
    if (
      requestId !== this.requestGeneration ||
      (this.phaseValue !== 'loading' && this.phaseValue !== 'settling')
    ) {
      return undefined;
    }
    return this.failActiveNavigation(reason);
  }

  public cancelForUser(): TimelineNavigationFailure<TTimeline> | undefined {
    if (this.phaseValue !== 'loading' && this.phaseValue !== 'settling') return undefined;
    return this.failActiveNavigation('user-cancelled');
  }

  public completeSettlement(
    bottomConfirmed: boolean,
    clearedRouteKey?: string
  ): TimelineNavigationCompletion & { failure?: TimelineNavigationFailure<TTimeline> } {
    if (this.phaseValue !== 'settling') return { accepted: false };
    if (!bottomConfirmed) {
      return {
        accepted: true,
        failure: this.failActiveNavigation('settling-unconfirmed'),
      };
    }

    this.clearPhaseTimeout();
    const focusedEventId = this.focusedEventId;
    this.previousTimeline = undefined;
    this.focusedEventId = undefined;
    this.phaseValue = 'idle';
    if (focusedEventId && clearedRouteKey) this.expectedRouteKey = clearedRouteKey;
    return { accepted: true, focusedEventId };
  }

  public handleRouteChange(nextRouteKey: string): boolean {
    const previousRouteKey = this.routeKey;
    this.routeKey = nextRouteKey;
    if (previousRouteKey === nextRouteKey) return false;
    if (nextRouteKey === this.expectedRouteKey) {
      this.expectedRouteKey = undefined;
      return false;
    }

    this.requestGeneration += 1;
    this.invalidateLiveTailRefresh();
    this.clearPhaseTimeout();
    this.previousTimeline = undefined;
    this.focusedEventId = undefined;
    this.authoritativeTailEventIdValue = undefined;
    this.expectedRouteKey = undefined;
    this.phaseValue = 'idle';
    return true;
  }

  public beginLiveTailRefresh(): number {
    this.liveTailRefreshGeneration += 1;
    return this.liveTailRefreshGeneration;
  }

  public canApplyLiveTailRefresh(requestId: number): boolean {
    return (
      requestId === this.liveTailRefreshGeneration &&
      this.phaseValue !== 'loading' &&
      this.phaseValue !== 'settling'
    );
  }

  public applyLiveTailRefresh(requestId: number, authoritativeTailEventId: string): boolean {
    if (!this.canApplyLiveTailRefresh(requestId)) return false;
    this.authoritativeTailEventIdValue = authoritativeTailEventId;
    return true;
  }

  public invalidateLiveTailRefresh(): void {
    this.liveTailRefreshGeneration += 1;
  }

  public reattachLiveTimeline(): void {
    this.invalidateLiveTailRefresh();
    this.authoritativeTailEventIdValue = undefined;
  }

  public getBounds(
    currentWindowTailEventId: string | undefined,
    liveTimelineLinked: boolean,
    hasForwardPaginationToken: boolean
  ): TimelineNavigationBounds {
    const authoritativeLatestWindow = Boolean(
      this.authoritativeTailEventIdValue &&
        currentWindowTailEventId === this.authoritativeTailEventIdValue
    );
    const canPaginateForward = !authoritativeLatestWindow && hasForwardPaginationToken;
    return {
      authoritativeLatestWindow,
      canPaginateForward,
      loadedAtEnd: authoritativeLatestWindow || (liveTimelineLinked && !canPaginateForward),
    };
  }

  public getPersistedLiveTailEventId(
    loadedLiveTailEventId: string | undefined
  ): string | undefined {
    return this.authoritativeTailEventIdValue ?? loadedLiveTailEventId;
  }

  public dispose(): void {
    this.requestGeneration += 1;
    this.invalidateLiveTailRefresh();
    this.clearPhaseTimeout();
    this.previousTimeline = undefined;
    this.focusedEventId = undefined;
    this.authoritativeTailEventIdValue = undefined;
    this.expectedRouteKey = undefined;
    this.phaseValue = 'idle';
  }

  private failActiveNavigation(
    reason: TimelineNavigationFailureReason
  ): TimelineNavigationFailure<TTimeline> {
    this.requestGeneration += 1;
    this.clearPhaseTimeout();
    const previousTimeline = this.previousTimeline;
    this.previousTimeline = undefined;
    this.focusedEventId = undefined;
    this.authoritativeTailEventIdValue = undefined;
    this.expectedRouteKey = undefined;
    this.phaseValue = 'error';
    return { reason, previousTimeline };
  }

  private schedulePhaseTimeout(
    phase: Extract<TimelineNavigationPhase, 'loading' | 'settling'>,
    requestId: number
  ): void {
    this.clearPhaseTimeout();
    const timeoutMs = phase === 'loading' ? this.loadingTimeoutMs : this.settlingTimeoutMs;
    this.timeoutHandle = this.scheduler.schedule(() => {
      this.timeoutHandle = undefined;
      if (requestId !== this.requestGeneration || this.phaseValue !== phase) return;
      const reason = phase === 'loading' ? 'loading-timeout' : 'settling-timeout';
      this.onTimeout(this.failActiveNavigation(reason));
    }, timeoutMs);
  }

  private clearPhaseTimeout(): void {
    if (this.timeoutHandle === undefined) return;
    this.scheduler.cancel(this.timeoutHandle);
    this.timeoutHandle = undefined;
  }
}
