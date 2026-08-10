/**
 * PROHIBITED FIXTURE — P1.6 guardrail must reject this file.
 * Raw /_matrix/ product client HTTP outside approved exception paths.
 */
export async function fetchVersions(baseUrl: string): Promise<Response> {
  return fetch(`${baseUrl}/_matrix/client/versions`);
}
