import assert from 'node:assert/strict';
import test from 'node:test';
import {
  FOUNDATION_FEATURE_DEFINITIONS,
  resolveFoundationFeatureFlags,
} from '../foundationFeatures';

test('desktop foundation rollout features default enabled', () => {
  assert.deepEqual(resolveFoundationFeatureFlags({ buildEnv: {}, runtimeOverrides: {} }), {
    exactReadMarkers: true,
    reactiveRoomActivity: true,
    boundedTimelineContexts: true,
    stableScrollAnchoring: true,
  });
});

test('build flags can emergency-disable each desktop foundation feature', () => {
  const buildEnv = Object.fromEntries(
    Object.values(FOUNDATION_FEATURE_DEFINITIONS).map((definition) => [
      definition.buildEnv,
      'false',
    ])
  );

  assert.deepEqual(resolveFoundationFeatureFlags({ buildEnv, runtimeOverrides: {} }), {
    exactReadMarkers: false,
    reactiveRoomActivity: false,
    boundedTimelineContexts: false,
    stableScrollAnchoring: false,
  });
});

test('explicit runtime values override builds and invalid values are ignored', () => {
  assert.deepEqual(
    resolveFoundationFeatureFlags({
      buildEnv: {
        VITE_SYNARA_FEATURE_EXACT_READ_MARKERS: 'false',
        VITE_SYNARA_FEATURE_REACTIVE_ROOM_ACTIVITY: 'false',
      },
      runtimeOverrides: {
        exactReadMarkers: 'true',
        reactiveRoomActivity: 'invalid',
        boundedTimelineContexts: 'false',
      },
    }),
    {
      exactReadMarkers: true,
      reactiveRoomActivity: false,
      boundedTimelineContexts: false,
      stableScrollAnchoring: true,
    }
  );
});
