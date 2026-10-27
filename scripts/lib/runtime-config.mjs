/**
 * Build-time additions to the bundled runtime config.json.
 *
 * The GIF provider key ships inside the app, as client-side GIF keys do, but
 * it never enters the repository: config.json keeps `apiKey` empty and the
 * build fills it from SYNARA_KLIPY_API_KEY into devAssets/config.json, which
 * git ignores.
 */
export const KLIPY_API_KEY_ENV = "SYNARA_KLIPY_API_KEY";

const KEY_PATTERN = /^[A-Za-z0-9_-]{8,128}$/;

export function withGifApiKey(config, key) {
  const trimmed = typeof key === "string" ? key.trim() : "";
  if (!trimmed) return config;
  if (!KEY_PATTERN.test(trimmed)) {
    throw new Error(`${KLIPY_API_KEY_ENV} is not a valid API key.`);
  }
  const gifPicker = config.gifPicker;
  if (!gifPicker || gifPicker.provider !== "klipy") return config;
  return { ...config, gifPicker: { ...gifPicker, apiKey: trimmed } };
}
