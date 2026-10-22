/**
 * Whether media URLs should use authenticated media. Core fetches every
 * `mxc://` through the `synara-media` protocol with the session's own
 * authentication, and `mxcUrlToHttp` ignores this flag, so the renderer no
 * longer asks the homeserver's /versions for it (D83).
 */
export const useMediaAuthentication = (): boolean => true;
