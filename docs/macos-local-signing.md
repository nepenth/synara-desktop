# macOS Local Signing And Notarization

This is the preferred setup while Synara Desktop is built and installed only by
the project maintainers on local Macs.

GitHub Actions secrets are only needed for CI/release builds on GitHub runners.
Local builds use the Developer ID Application certificate in the local Keychain
and Apple notarization credentials from a local, ignored env file.

## One-Time Setup

1. Install the Developer ID Application certificate into the `login` keychain.
2. Confirm it appears under Keychain Access > login > My Certificates and has a
   private key nested underneath it.
3. Confirm the command-line tools can see it:

   ```bash
   security find-identity -v -p codesigning | grep "Developer ID Application"
   ```

4. Create the local env file:

   ```bash
   cp .env.macos-signing.example .env.macos-signing.local
   ```

5. Edit `.env.macos-signing.local` with:
   - `APPLE_TEAM_ID`
   - `APPLE_SIGNING_IDENTITY`
   - `APPLE_ID`
   - `APPLE_APP_SPECIFIC_PASSWORD`

`.env.macos-signing.local` is ignored by git. Do not commit it.

## Build

Run:

```bash
npm run build:macos:local
```

The command builds a universal macOS DMG, signs with the local Developer ID
identity, submits notarization through Tauri, staples the result, and verifies:

```bash
codesign --verify --deep --strict
spctl --assess
xcrun stapler validate
```

Outputs are under:

```text
src-tauri/target/universal-apple-darwin/release/bundle/
```

## Troubleshooting

If the script says the Developer ID identity is not visible, fix Keychain first:

- Put the certificate in `login`, not iCloud.
- Use Keychain Access > login > My Certificates.
- Confirm the certificate has a private key underneath it.
- If needed, lock/unlock the login keychain or restart Terminal/Codex.
