import assert from 'node:assert/strict';
import test from 'node:test';
import { isSafeWidgetUrl } from '../widgetUrl';

test('public https is safe for room-state and agent widgets', () => {
  assert.equal(isSafeWidgetUrl('https://widgets.example.org/app', false), true);
  assert.equal(isSafeWidgetUrl('https://widgets.example.org/app', true), true);
});

test('file javascript data and credentials are rejected', () => {
  assert.equal(isSafeWidgetUrl('file:///etc/passwd', false), false);
  assert.equal(isSafeWidgetUrl('javascript:alert(1)', true), false);
  assert.equal(isSafeWidgetUrl('data:text/html,hi', false), false);
  assert.equal(isSafeWidgetUrl('https://user:pass@widgets.example.org/app', true), false);
});

test('token query markers are rejected', () => {
  assert.equal(
    isSafeWidgetUrl('https://widgets.example.org/app?access_token=secret', false),
    false
  );
  assert.equal(
    isSafeWidgetUrl('https://widgets.example.org/app?loginToken=secret', true),
    false
  );
});

test('loopback is agent-list only', () => {
  for (const url of [
    'http://127.0.0.1:8765/agent',
    'http://localhost:3000/',
    'http://[::1]:8080/',
  ]) {
    assert.equal(isSafeWidgetUrl(url, true), true, url);
    assert.equal(isSafeWidgetUrl(url, false), false, url);
  }
});

test('lan and private hosts are never safe', () => {
  for (const url of [
    'https://10.0.0.5/widget',
    'https://192.168.1.10/widget',
    'https://app.local/widget',
  ]) {
    assert.equal(isSafeWidgetUrl(url, false), false, url);
    assert.equal(isSafeWidgetUrl(url, true), false, url);
  }
});
