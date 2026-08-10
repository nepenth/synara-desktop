import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const read = (relativePath: string): string => readFileSync(relativePath, 'utf8');

test('async search invalidates stale results after commit, not during render', () => {
  const source = read('src/app/hooks/useAsyncSearch.ts');
  const factoryStart = source.indexOf('const [searchCallback, terminateSearch] = useMemo');
  const invalidationEffect = source.indexOf('useEffect(() => {', factoryStart);

  assert.notEqual(factoryStart, -1, 'expected the memoized AsyncSearch factory');
  assert.notEqual(invalidationEffect, -1, 'expected post-commit result invalidation');
  assert.doesNotMatch(
    source.slice(factoryStart, invalidationEffect),
    /setResult\(undefined\)/,
    'the memoized factory must not clear result during render'
  );

  const effect = source.slice(
    invalidationEffect,
    source.indexOf('const searchHandler', invalidationEffect)
  );
  assert.match(effect, /setResult\(undefined\)/);
  assert.match(effect, /\[terminateSearch\]/);
});

test('member search restoration invokes search from an effect, never the render body', () => {
  for (const path of [
    'src/app/features/room/MembersDrawer.tsx',
    'src/app/features/common-settings/members/Members.tsx',
  ]) {
    const source = read(path);
    const asyncSearchCall = source.indexOf('const [result, search, resetSearch] = useAsyncSearch');
    const restorationEffect = source.indexOf('useEffect(() => {', asyncSearchCall);

    assert.notEqual(asyncSearchCall, -1, `expected async search in ${path}`);
    assert.notEqual(restorationEffect, -1, `expected restoration effect in ${path}`);
    assert.doesNotMatch(
      source.slice(asyncSearchCall, restorationEffect),
      /searchInputRef\.current\?\.value\) search\(/,
      `${path} must not invoke search during render`
    );
    assert.match(
      source.slice(restorationEffect),
      /useEffect\(\(\) => \{[\s\S]*?searchInputRef\.current\?\.value\) search\(/
    );
  }
});

test('native creator loading returns a stable empty set for memoized member consumers', () => {
  const source = read('src/app/hooks/useRoomCreators.ts');

  assert.match(source, /const emptyCreators = useMemo\(\(\) => new Set<string>\(\), \[\]\)/);
  assert.match(source, /nativeState\.status !== 'ready'\) \{\n\s+return emptyCreators/);
});
