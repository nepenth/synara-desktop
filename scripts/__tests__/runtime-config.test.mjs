import assert from "node:assert/strict";
import test from "node:test";
import { withGifApiKey } from "../lib/runtime-config.mjs";

const config = {
  defaultHomeserver: 1,
  gifPicker: { enabled: true, provider: "klipy", apiKey: "" },
};

test("the Klipy key fills only the bundled gifPicker", () => {
  const keyed = withGifApiKey(config, " key_ABC-123 ");
  assert.equal(keyed.gifPicker.apiKey, "key_ABC-123");
  assert.equal(config.gifPicker.apiKey, "", "the source config is untouched");
  assert.equal(keyed.defaultHomeserver, 1);
});

test("no key, or another provider, leaves the config as it is", () => {
  assert.equal(withGifApiKey(config, undefined), config);
  assert.equal(withGifApiKey(config, "  "), config);
  const giphy = { gifPicker: { provider: "giphy", apiKey: "" } };
  assert.equal(withGifApiKey(giphy, "key_ABC-123"), giphy);
});

test("a malformed key fails the build instead of shipping", () => {
  assert.throws(() => withGifApiKey(config, "bad key\""), /not a valid API key/);
});
