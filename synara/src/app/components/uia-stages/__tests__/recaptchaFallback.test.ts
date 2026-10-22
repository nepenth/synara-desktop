import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { recaptchaFallbackUrl } from '../recaptchaFallback';

test('the CAPTCHA stage opens the homeserver fallback page for this session', () => {
  assert.equal(
    recaptchaFallbackUrl('https://matrix-client.matrix.org/', 'abc/def'),
    'https://matrix-client.matrix.org/_matrix/client/v3/auth/m.login.recaptcha/fallback/web?session=abc%2Fdef'
  );
  assert.equal(
    recaptchaFallbackUrl('http://localhost:8008', 's1'),
    'http://localhost:8008/_matrix/client/v3/auth/m.login.recaptcha/fallback/web?session=s1'
  );
  // Plain HTTP to a remote host and an empty session are refused.
  assert.equal(recaptchaFallbackUrl('http://matrix.example.org', 's1'), undefined);
  assert.equal(recaptchaFallbackUrl('https://matrix.example.org', ''), undefined);
});

test('the CAPTCHA stage no longer embeds Google reCAPTCHA and continues with the session', () => {
  const stage = readFileSync('src/app/components/uia-stages/ReCaptchaStage.tsx', 'utf8');
  assert.ok(!stage.includes('react-google-recaptcha'));
  assert.ok(stage.includes("submitAuthDict({ type: 'session_only', session })"));
  const form = readFileSync('src/app/pages/auth/register/PasswordRegisterForm.tsx', 'utf8');
  assert.ok(form.includes("return { type: 'session_only', session: dict.session };"));
  assert.ok(form.includes('homeserverBaseUrl={baseUrl}'));
});
