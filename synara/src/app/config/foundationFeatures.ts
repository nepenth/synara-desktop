export const FOUNDATION_FEATURE_DEFINITIONS = {
  exactReadMarkers: {
    buildEnv: 'VITE_SYNARA_FEATURE_EXACT_READ_MARKERS',
    runtimeStorage: 'synara.feature.exactReadMarkers',
  },
  reactiveRoomActivity: {
    buildEnv: 'VITE_SYNARA_FEATURE_REACTIVE_ROOM_ACTIVITY',
    runtimeStorage: 'synara.feature.reactiveRoomActivity',
  },
  boundedTimelineContexts: {
    buildEnv: 'VITE_SYNARA_FEATURE_BOUNDED_TIMELINE_CONTEXTS',
    runtimeStorage: 'synara.feature.boundedTimelineContexts',
  },
  stableScrollAnchoring: {
    buildEnv: 'VITE_SYNARA_FEATURE_STABLE_SCROLL_ANCHORING',
    runtimeStorage: 'synara.feature.stableScrollAnchoring',
  },
} as const;

export type FoundationFeatureName = keyof typeof FOUNDATION_FEATURE_DEFINITIONS;
export type FoundationFeatureFlags = Record<FoundationFeatureName, boolean>;

type FoundationFeatureResolution = {
  buildEnv?: Record<string, string | undefined>;
  runtimeOverrides?: Partial<Record<FoundationFeatureName, string | null | undefined>>;
};

const importMetaEnv = (import.meta as ImportMeta & { env?: Record<string, string | undefined> })
  .env;

const parseBooleanOverride = (value: string | null | undefined): boolean | undefined => {
  if (value === 'true') return true;
  if (value === 'false') return false;
  return undefined;
};

const readRuntimeOverrides = (): FoundationFeatureResolution['runtimeOverrides'] => {
  if (typeof window === 'undefined') return undefined;
  try {
    return Object.fromEntries(
      Object.entries(FOUNDATION_FEATURE_DEFINITIONS).map(([name, definition]) => [
        name,
        window.localStorage.getItem(definition.runtimeStorage),
      ])
    ) as FoundationFeatureResolution['runtimeOverrides'];
  } catch {
    return undefined;
  }
};

export const resolveFoundationFeatureFlags = ({
  buildEnv = importMetaEnv ?? {},
  runtimeOverrides = readRuntimeOverrides(),
}: FoundationFeatureResolution = {}): FoundationFeatureFlags =>
  Object.fromEntries(
    Object.entries(FOUNDATION_FEATURE_DEFINITIONS).map(([name, definition]) => {
      const featureName = name as FoundationFeatureName;
      const runtimeValue = parseBooleanOverride(runtimeOverrides?.[featureName]);
      const buildValue = parseBooleanOverride(buildEnv[definition.buildEnv]);
      return [featureName, runtimeValue ?? buildValue ?? true];
    })
  ) as FoundationFeatureFlags;

export const isFoundationFeatureEnabled = (name: FoundationFeatureName): boolean =>
  resolveFoundationFeatureFlags()[name];
