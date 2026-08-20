import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { normalizeAgentActionPayload } from '../../agents/agentActions';
import { summarizeNotifications } from '../../notifications/badgeSummary';
import { normalizeSynaraRoute, parseSynaraRouteDestination } from '../../routes/synaraRoutes';
import { defaultDesktopPlatformSettings, defaultSharedSettings } from '../../state/settings';
import { parseHermesAgentPayload } from '../../utils/hermes';
import { normalizeLaterContent } from '../../utils/later';
import { safeRemoteContentUrl } from '../../utils/remoteContent';
import { normalizeRoomNotesContent } from '../../utils/roomNotes';

type JsonSchema = {
  $id?: string;
  $ref?: string;
  $defs?: Record<string, JsonSchema>;
  anyOf?: JsonSchema[];
  type?: string | string[];
  additionalProperties?: boolean | JsonSchema;
  required?: string[];
  properties?: Record<string, JsonSchema>;
  enum?: unknown[];
  const?: unknown;
  minLength?: number;
  maxLength?: number;
  minimum?: number;
  pattern?: string;
  items?: JsonSchema;
  minItems?: number;
  maxItems?: number;
  uniqueItems?: boolean;
};

type LaterFixtures = {
  valid: Record<string, unknown>;
  invalid: Record<string, unknown>;
  normalization: {
    legacyPlaintextFields: {
      input: unknown;
      normalized: unknown;
    };
  };
};

type AgentActionFixtures = {
  valid: Record<string, unknown>;
  invalid: Record<string, unknown>;
  normalization: Record<
    string,
    {
      input: unknown;
      normalized: unknown;
    }
  >;
};

type RouteFixtures = {
  valid: Record<string, string>;
  invalid: Record<string, string>;
  semanticInvalid: Record<string, string>;
  destinations: Record<
    string,
    {
      route: string;
      destination: unknown;
    }
  >;
};

type NotificationSummaryFixtures = {
  valid: Record<string, unknown>;
  invalid: Record<string, unknown>;
  runtime: Record<
    string,
    {
      input: Parameters<typeof summarizeNotifications>[0];
      summary: unknown;
    }
  >;
};

type FlatFixtures = {
  valid: Record<string, unknown>;
  invalid: Record<string, unknown>;
};

type SafeRemoteUrlFixtures = {
  valid: Record<string, string>;
  invalid: Record<string, string>;
};

type SettingsFixtures = {
  valid: {
    shared: unknown;
    sharedWithOptionalFields: unknown;
    desktopPlatform: unknown;
  };
  invalid: {
    sharedContainsDesktopShortcut: unknown;
    badMessageSpacing: unknown;
    badThemeBaseColor: unknown;
    desktopPlatformContainsSharedField: unknown;
  };
};

type NormalizationFixtures = {
  valid: Record<string, unknown>;
  invalid: Record<string, unknown>;
  normalization: Record<
    string,
    {
      input: unknown;
      normalized: unknown;
    }
  >;
};

type AgentCardFixtures = {
  valid: Record<string, unknown>;
  invalid: Record<string, unknown>;
  runtime: Record<
    string,
    {
      content: Record<string, unknown>;
      parsed: unknown;
    }
  >;
};

const readJson = <T>(path: string): T => JSON.parse(readFileSync(path, { encoding: 'utf8' })) as T;

const isPlainObject = (value: unknown): value is Record<string, unknown> =>
  !!value && typeof value === 'object' && !Array.isArray(value);

const jsonEqual = (a: unknown, b: unknown): boolean => JSON.stringify(a) === JSON.stringify(b);

const resolveRef = (schema: JsonSchema, root: JsonSchema): JsonSchema => {
  if (!schema.$ref) return schema;
  const match = schema.$ref.match(/^#\/\$defs\/([^/]+)$/);
  if (!match) throw new Error(`Unsupported schema ref: ${schema.$ref}`);
  const resolved = root.$defs?.[match[1]];
  if (!resolved) throw new Error(`Missing schema ref: ${schema.$ref}`);
  return resolved;
};

const matchesType = (value: unknown, type: string): boolean => {
  switch (type) {
    case 'object':
      return isPlainObject(value);
    case 'string':
      return typeof value === 'string';
    case 'number':
      return typeof value === 'number' && Number.isFinite(value);
    case 'integer':
      return typeof value === 'number' && Number.isInteger(value);
    case 'boolean':
      return typeof value === 'boolean';
    case 'array':
      return Array.isArray(value);
    case 'null':
      return value === null;
    default:
      throw new Error(`Unsupported schema type: ${type}`);
  }
};

const validateWithSchema = (
  value: unknown,
  schema: JsonSchema,
  root = schema,
  path = '$'
): string[] => {
  const resolved = resolveRef(schema, root);
  const errors: string[] = [];

  if (
    Object.prototype.hasOwnProperty.call(resolved, 'const') &&
    !jsonEqual(value, resolved.const)
  ) {
    errors.push(`${path} must equal ${JSON.stringify(resolved.const)}`);
  }

  if (resolved.enum && !resolved.enum.some((option) => jsonEqual(value, option))) {
    errors.push(`${path} must be one of ${JSON.stringify(resolved.enum)}`);
  }

  const types = Array.isArray(resolved.type) ? resolved.type : resolved.type ? [resolved.type] : [];
  if (types.length > 0 && !types.some((type) => matchesType(value, type))) {
    errors.push(`${path} must be ${types.join(' or ')}`);
    return errors;
  }

  if (resolved.anyOf) {
    const matchingSchemas = resolved.anyOf.filter(
      (candidate) => validateWithSchema(value, candidate, root, path).length === 0
    );
    if (matchingSchemas.length === 0) {
      errors.push(`${path} must match at least one allowed schema shape`);
    }
  }

  if (typeof value === 'string') {
    if (resolved.minLength !== undefined && value.length < resolved.minLength) {
      errors.push(`${path} must have at least ${resolved.minLength} characters`);
    }
    if (resolved.maxLength !== undefined && value.length > resolved.maxLength) {
      errors.push(`${path} must have at most ${resolved.maxLength} characters`);
    }
    if (resolved.pattern && !new RegExp(resolved.pattern).test(value)) {
      errors.push(`${path} must match ${resolved.pattern}`);
    }
  }

  if (typeof value === 'number' && resolved.minimum !== undefined && value < resolved.minimum) {
    errors.push(`${path} must be at least ${resolved.minimum}`);
  }

  if (Array.isArray(value)) {
    if (resolved.minItems !== undefined && value.length < resolved.minItems) {
      errors.push(`${path} must have at least ${resolved.minItems} items`);
    }
    if (resolved.maxItems !== undefined && value.length > resolved.maxItems) {
      errors.push(`${path} must have at most ${resolved.maxItems} items`);
    }
    if (resolved.uniqueItems) {
      const seen = new Set(value.map((item) => JSON.stringify(item)));
      if (seen.size !== value.length) errors.push(`${path} must have unique items`);
    }
    if (resolved.items) {
      value.forEach((item, index) => {
        errors.push(...validateWithSchema(item, resolved.items!, root, `${path}[${index}]`));
      });
    }
    return errors;
  }

  if (!isPlainObject(value)) return errors;

  const properties = resolved.properties ?? {};
  (resolved.required ?? []).forEach((field) => {
    if (!Object.prototype.hasOwnProperty.call(value, field)) {
      errors.push(`${path}.${field} is required`);
    }
  });

  Object.entries(properties).forEach(([field, fieldSchema]) => {
    if (Object.prototype.hasOwnProperty.call(value, field)) {
      errors.push(...validateWithSchema(value[field], fieldSchema, root, `${path}.${field}`));
    }
  });

  Object.entries(value).forEach(([field, fieldValue]) => {
    if (Object.prototype.hasOwnProperty.call(properties, field)) return;
    if (resolved.additionalProperties === false) {
      errors.push(`${path}.${field} is not allowed`);
      return;
    }
    if (isPlainObject(resolved.additionalProperties)) {
      errors.push(
        ...validateWithSchema(fieldValue, resolved.additionalProperties, root, `${path}.${field}`)
      );
    }
  });

  return errors;
};

const stripUndefined = (value: unknown): unknown => {
  if (Array.isArray(value)) return value.map(stripUndefined);
  if (!isPlainObject(value)) return value;
  return Object.fromEntries(
    Object.entries(value)
      .filter(([, fieldValue]) => fieldValue !== undefined)
      .map(([field, fieldValue]) => [field, stripUndefined(fieldValue)])
  );
};

const asStoredJson = (value: unknown): unknown => JSON.parse(JSON.stringify(value));

const assertValidFixtures = (
  fixtures: Record<string, unknown>,
  schema: JsonSchema,
  label: string
) => {
  Object.entries(fixtures).forEach(([name, fixture]) => {
    assert.deepEqual(validateWithSchema(fixture, schema), [], `${label}.${name} should be valid`);
  });
};

const assertInvalidFixtures = (
  fixtures: Record<string, unknown>,
  schema: JsonSchema,
  label: string
) => {
  Object.entries(fixtures).forEach(([name, fixture]) => {
    const errors = validateWithSchema(fixture, schema);
    assert.ok(errors.length > 0, `${label}.${name} should be invalid`);
  });
};

test('Later contract schema validates canonical fixtures and rejects non-canonical payloads', () => {
  const schema = readJson<JsonSchema>('docs/contracts/synara-later-content.schema.json');
  const fixtures = readJson<LaterFixtures>('docs/contracts/fixtures/synara-later-content.json');

  assert.equal(schema.$id, 'https://synara.local/contracts/synara-later-content.schema.json');
  assert.deepEqual(schema.$defs?.SynaraLaterItem.properties?.kind?.enum, ['saved', 'reminder']);

  assertValidFixtures(fixtures.valid, schema, 'later.valid');
  assertInvalidFixtures(fixtures.invalid, schema, 'later.invalid');

  assert.deepEqual(
    normalizeLaterContent(fixtures.normalization.legacyPlaintextFields.input as any),
    fixtures.normalization.legacyPlaintextFields.normalized
  );
});

test('Room notes account-data schema validates fixtures and runtime normalization', () => {
  const schema = readJson<JsonSchema>('docs/contracts/synara-room-notes-content.schema.json');
  const fixtures = readJson<NormalizationFixtures>(
    'docs/contracts/fixtures/synara-room-notes-content.json'
  );

  assert.equal(schema.$id, 'https://synara.local/contracts/synara-room-notes-content.schema.json');
  assertValidFixtures(fixtures.valid, schema, 'roomNotes.valid');
  assertInvalidFixtures(fixtures.invalid, schema, 'roomNotes.invalid');

  Object.entries(fixtures.normalization).forEach(([name, fixture]) => {
    assert.deepEqual(
      normalizeRoomNotesContent(fixture.input as any),
      fixture.normalized,
      `${name} should normalize through runtime helper`
    );
  });
});

test('Unread anchor account-data schema validates fixtures', () => {
  const schema = readJson<JsonSchema>('docs/contracts/synara-unread-anchor-content.schema.json');
  const fixtures = readJson<FlatFixtures>(
    'docs/contracts/fixtures/synara-unread-anchor-content.json'
  );

  assert.equal(
    schema.$id,
    'https://synara.local/contracts/synara-unread-anchor-content.schema.json'
  );
  assertValidFixtures(fixtures.valid, schema, 'unreadAnchor.valid');
  assertInvalidFixtures(fixtures.invalid, schema, 'unreadAnchor.invalid');
});

test('Spaces/sidebar account-data schema validates folder fixtures', () => {
  const schema = readJson<JsonSchema>('docs/contracts/synara-spaces-content.schema.json');
  const fixtures = readJson<FlatFixtures>('docs/contracts/fixtures/synara-spaces-content.json');

  assert.equal(schema.$id, 'https://synara.local/contracts/synara-spaces-content.schema.json');
  assertValidFixtures(fixtures.valid, schema, 'spaces.valid');
  assertInvalidFixtures(fixtures.invalid, schema, 'spaces.invalid');
});

test('Agent-card schema validates fixtures and runtime parsing', () => {
  const schema = readJson<JsonSchema>('docs/contracts/synara-agent-card.schema.json');
  const fixtures = readJson<AgentCardFixtures>('docs/contracts/fixtures/synara-agent-card.json');

  assert.equal(schema.$id, 'https://synara.local/contracts/synara-agent-card.schema.json');
  assertValidFixtures(fixtures.valid, schema, 'agentCard.valid');
  assertInvalidFixtures(fixtures.invalid, schema, 'agentCard.invalid');

  Object.entries(fixtures.runtime).forEach(([name, fixture]) => {
    assert.deepEqual(
      stripUndefined(parseHermesAgentPayload(fixture.content)),
      fixture.parsed,
      `${name} should parse through runtime agent-card helper`
    );
  });
});

test('Agent action contract schema validates fixtures and runtime normalization', () => {
  const schema = readJson<JsonSchema>('docs/contracts/synara-agent-action.schema.json');
  const fixtures = readJson<AgentActionFixtures>(
    'docs/contracts/fixtures/synara-agent-action.json'
  );

  assert.equal(schema.$id, 'https://synara.local/contracts/synara-agent-action.schema.json');
  assert.deepEqual(schema.properties?.kind?.enum, [
    'agent',
    'copy',
    'continue',
    'export',
    'prompt',
    'regenerate',
    'run',
    'approve',
    'reject',
    'open',
    'open_url',
  ]);

  assertValidFixtures(fixtures.valid, schema, 'agentAction.valid');
  assertInvalidFixtures(fixtures.invalid, schema, 'agentAction.invalid');

  Object.entries(fixtures.normalization).forEach(([name, fixture]) => {
    assert.deepEqual(
      stripUndefined(normalizeAgentActionPayload(fixture.input as any)),
      fixture.normalized,
      `${name} should normalize through runtime validator`
    );
  });
});

test('Agent approval action schema validates result fixtures', () => {
  const schema = readJson<JsonSchema>('docs/contracts/synara-agent-approval-action.schema.json');
  const fixtures = readJson<{
    valid: Record<string, unknown>;
    invalid: Record<string, unknown>;
  }>('docs/contracts/fixtures/synara-agent-approval-action.json');

  assert.equal(
    schema.$id,
    'https://synara.local/contracts/synara-agent-approval-action.schema.json'
  );
  assertValidFixtures(fixtures.valid, schema, 'agentApproval.valid');
  assertInvalidFixtures(fixtures.invalid, schema, 'agentApproval.invalid');
});

test('Route contract schema validates fixtures and runtime destination parsing', () => {
  const schema = readJson<JsonSchema>('docs/contracts/synara-route.schema.json');
  const fixtures = readJson<RouteFixtures>('docs/contracts/fixtures/synara-route.json');

  assert.equal(schema.$id, 'https://synara.local/contracts/synara-route.schema.json');
  assertValidFixtures(fixtures.valid, schema, 'route.valid');
  assertInvalidFixtures(fixtures.invalid, schema, 'route.invalid');

  Object.entries(fixtures.valid).forEach(([name, route]) => {
    assert.equal(normalizeSynaraRoute(route), route, `${name} should normalize`);
  });
  Object.entries({ ...fixtures.invalid, ...fixtures.semanticInvalid }).forEach(([name, route]) => {
    assert.equal(normalizeSynaraRoute(route), undefined, `${name} should fail closed`);
  });
  Object.entries(fixtures.destinations).forEach(([name, fixture]) => {
    assert.deepEqual(
      parseSynaraRouteDestination(fixture.route),
      fixture.destination,
      `${name} destination should parse`
    );
  });
});

test('Notification summary contract schema validates fixtures and runtime formulas', () => {
  const schema = readJson<JsonSchema>('docs/contracts/synara-notification-summary.schema.json');
  const fixtures = readJson<NotificationSummaryFixtures>(
    'docs/contracts/fixtures/synara-notification-summary.json'
  );

  assert.equal(
    schema.$id,
    'https://synara.local/contracts/synara-notification-summary.schema.json'
  );
  assertValidFixtures(fixtures.valid, schema, 'notificationSummary.valid');
  assertInvalidFixtures(fixtures.invalid, schema, 'notificationSummary.invalid');

  Object.entries(fixtures.runtime).forEach(([name, fixture]) => {
    assert.deepEqual(
      summarizeNotifications(fixture.input),
      fixture.summary,
      `${name} summary should match formula`
    );
    assert.deepEqual(
      validateWithSchema(fixture.summary, schema),
      [],
      `${name} runtime summary should be schema-valid`
    );
  });
});

test('Room/event anchor contract schema validates opaque Matrix anchor fixtures', () => {
  const schema = readJson<JsonSchema>('docs/contracts/synara-room-event-anchor.schema.json');
  const fixtures = readJson<FlatFixtures>('docs/contracts/fixtures/synara-room-event-anchor.json');

  assert.equal(schema.$id, 'https://synara.local/contracts/synara-room-event-anchor.schema.json');
  assertValidFixtures(fixtures.valid, schema, 'anchor.valid');
  assertInvalidFixtures(fixtures.invalid, schema, 'anchor.invalid');
});

test('Safe remote URL contract schema tracks runtime URL safety fixtures', () => {
  const schema = readJson<JsonSchema>('docs/contracts/synara-safe-remote-url.schema.json');
  const fixtures = readJson<SafeRemoteUrlFixtures>(
    'docs/contracts/fixtures/synara-safe-remote-url.json'
  );

  assert.equal(schema.$id, 'https://synara.local/contracts/synara-safe-remote-url.schema.json');
  assertValidFixtures(fixtures.valid, schema, 'safeRemoteUrl.valid');
  assertInvalidFixtures(fixtures.invalid, schema, 'safeRemoteUrl.invalid');

  Object.entries(fixtures.valid).forEach(([name, url]) => {
    assert.equal(safeRemoteContentUrl(url), url, `${name} should be runtime-safe`);
  });
  Object.entries(fixtures.invalid).forEach(([name, url]) => {
    assert.equal(safeRemoteContentUrl(url), undefined, `${name} should fail closed`);
  });
});

test('Settings compatibility schemas validate shared and desktop platform split', () => {
  const sharedSchema = readJson<JsonSchema>('docs/contracts/synara-shared-settings.schema.json');
  const desktopSchema = readJson<JsonSchema>(
    'docs/contracts/synara-desktop-platform-settings.schema.json'
  );
  const fixtures = readJson<SettingsFixtures>('docs/contracts/fixtures/synara-settings.json');

  assert.equal(
    sharedSchema.$id,
    'https://synara.local/contracts/synara-shared-settings.schema.json'
  );
  assert.equal(
    desktopSchema.$id,
    'https://synara.local/contracts/synara-desktop-platform-settings.schema.json'
  );

  assert.deepEqual(validateWithSchema(asStoredJson(defaultSharedSettings), sharedSchema), []);
  assert.deepEqual(
    validateWithSchema(asStoredJson(defaultDesktopPlatformSettings), desktopSchema),
    []
  );
  assert.deepEqual(validateWithSchema(fixtures.valid.shared, sharedSchema), []);
  assert.deepEqual(validateWithSchema(fixtures.valid.sharedWithOptionalFields, sharedSchema), []);
  assert.deepEqual(validateWithSchema(fixtures.valid.desktopPlatform, desktopSchema), []);

  assert.ok(
    validateWithSchema(fixtures.invalid.sharedContainsDesktopShortcut, sharedSchema).length > 0
  );
  assert.ok(validateWithSchema(fixtures.invalid.badMessageSpacing, sharedSchema).length > 0);
  assert.ok(validateWithSchema(fixtures.invalid.badThemeBaseColor, sharedSchema).length > 0);
  assert.ok(
    validateWithSchema(fixtures.invalid.desktopPlatformContainsSharedField, desktopSchema).length >
      0
  );
});
