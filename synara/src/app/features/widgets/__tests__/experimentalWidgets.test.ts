import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { defaultSettings } from '../../../state/settings';
import { isSafeWidgetUrl } from '../widgetUrl';
import {
  WIDGETS_SETTINGS_PATH,
  widgetsAgentHowToCopy,
  widgetsPanelEmptyCopy,
  widgetsSettingsDescription,
} from '../experimentalWidgets';

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
  assert.equal(isSafeWidgetUrl('https://widgets.example.org/app?loginToken=secret', true), false);
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

test('experimental widgets stay off by default and are not account data', () => {
  assert.equal(defaultSettings.experimentalWidgetsEnabled, false);
  assert.deepEqual(defaultSettings.agentWidgetEntries, []);
  const settingsSource = readFileSync('src/app/state/settings.ts', 'utf8');
  assert.doesNotMatch(settingsSource, /setAccountData/);
  assert.doesNotMatch(settingsSource, /account_data/);
});

test('widget empty-state copy points at Advanced › Experimental and explains agents', () => {
  assert.equal(
    widgetsPanelEmptyCopy(false),
    `Experimental Widgets are off. Enable them in ${WIDGETS_SETTINGS_PATH}.`
  );
  assert.match(
    widgetsPanelEmptyCopy(true),
    /Add an agent URL in Settings → Advanced → Experimental/
  );
  assert.match(widgetsPanelEmptyCopy(true), /m\.widget/);
  assert.match(widgetsAgentHowToCopy(), /http:\/\/127\.0\.0\.1/);
  assert.match(widgetsAgentHowToCopy(), /m\.room\.message/);
  assert.match(widgetsSettingsDescription, /Default off/);
  assert.match(widgetsSettingsDescription, /This device only/);
  assert.match(widgetsSettingsDescription, /not synced to the account or other devices/);
});

test('Advanced › Experimental owns the Widgets editor, and only one page has it', () => {
  const advanced = readFileSync('src/app/features/settings/advanced/Advanced.tsx', 'utf8');
  const general = readFileSync('src/app/features/settings/general/General.tsx', 'utf8');
  const panel = readFileSync('src/app/features/room/RoomWidgetsPanel.tsx', 'utf8');
  assert.match(advanced, /title="Experimental widgets"/);
  assert.match(advanced, /experimentalWidgetsEnabled/);
  assert.match(advanced, /agentWidgetEntries/);
  assert.match(advanced, /Add agent widget/);
  assert.match(advanced, /widgetsSettingsDescription/);
  assert.match(advanced, /closeExperimentalWidgets/);
  assert.equal(general.includes('experimentalWidgetsEnabled'), false);
  assert.equal(general.includes('Add agent widget'), false);
  assert.match(panel, /widgetsPanelEmptyCopy\(experimentalWidgetsEnabled\)/);
  assert.match(panel, /widgetsAgentHowToCopy\(\)/);
  const host = readFileSync('src/app/features/widgets/experimentalWidgets.ts', 'utf8');
  assert.doesNotMatch(host, /rtc_transports/);
});
