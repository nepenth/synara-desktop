import assert from 'node:assert/strict';
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';
import { FOLDS_PROGRESS_PACKET_CLASS, FOLDS_SPINNER_SVG_CLASS } from '../../utils/motion';

const source = (path: string) => readFileSync(path, 'utf8');

const withoutComments = (text: string) =>
  text.replace(/\/\*[\s\S]*?\*\//g, '').replace(/^\s*\/\/.*$/gm, '');

const sourceFiles = (dir: string): string[] =>
  readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) return entry.name === '__tests__' ? [] : sourceFiles(path);
    return /\.(tsx?|css)$/.test(entry.name) ? [path] : [];
  });

test('every infinite animation in the app goes through the motion budget', () => {
  const unguarded: string[] = [];
  for (const path of sourceFiles('src')) {
    const code = withoutComments(source(path));
    const infinite = code.match(/\binfinite\b/g)?.length ?? 0;
    if (infinite === 0) continue;
    // One continuousMotion() per infinite animation: it pauses the animation
    // in an inactive window and steps it in safe rendering mode.
    const guarded = code.match(/\bcontinuousMotion\(/g)?.length ?? 0;
    if (guarded < infinite) unguarded.push(`${path} (${infinite} infinite, ${guarded} guarded)`);
  }
  assert.deepEqual(unguarded, [], 'wrap infinite animations in continuousMotion (utils/motion.ts)');
});

test('the message highlight pulses a fixed number of times', () => {
  const layout = withoutComments(source('src/app/components/message/layout/layout.css.ts'));
  assert.match(layout, /animationIterationCount: 3/);
  assert.doesNotMatch(layout, /infinite/);
});

test("folds' infinite animations are exactly the ones motion.css.ts overrides", () => {
  const foldsCss = source('node_modules/folds/dist/style.css');
  const infiniteClasses = [...foldsCss.matchAll(/\.([\w-]+) \{([^}]*)\}/g)]
    .filter(([, , body]) => /animation:[^;]*\binfinite\b/.test(body))
    .map(([, name]) => name)
    .sort();
  assert.deepEqual(infiniteClasses, [FOLDS_SPINNER_SVG_CLASS, FOLDS_PROGRESS_PACKET_CLASS].sort());

  const motionCss = source('src/app/styles/motion.css.ts');
  assert.match(motionCss, /FOLDS_SPINNER_SVG_CLASS/);
  assert.match(motionCss, /FOLDS_PROGRESS_PACKET_CLASS/);
  assert.match(motionCss, /WINDOW_INACTIVE_SELECTOR/);
  assert.match(motionCss, /SAFE_RENDER_SELECTOR/);
  // The folds keyframes must not set their own timing function, or steps()
  // would not govern every interval and WebKit would tick every frame.
  for (const keyframes of foldsCss.matchAll(/@keyframes [\w-]+ \{([\s\S]*?\}\s*)\}/g)) {
    assert.doesNotMatch(keyframes[1], /animation-timing-function/);
  }
});

test('the policy is mounted once at the app root', () => {
  const app = source('src/app/pages/App.tsx');
  assert.match(app, /import '\.\.\/styles\/motion\.css';/);
  assert.match(app, /^\s+useMotionPolicy\(\);$/m);
  const callers = sourceFiles('src').filter(
    (path) => !path.endsWith('useMotionPolicy.ts') && /useMotionPolicy\(\)/.test(source(path))
  );
  assert.deepEqual(callers, [join('src', 'app', 'pages', 'App.tsx')]);
});
