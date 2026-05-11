import { existsSync } from 'node:fs';

const appPath = '/Applications/Synara.app';
const nestedAppPath = `${appPath}/Synara.app`;

if (process.platform !== 'darwin') {
  process.exit(0);
}

if (existsSync(nestedAppPath)) {
  console.warn(
    `[synara-desktop] Warning: ${nestedAppPath} exists. This usually means a previous build was copied into the existing app bundle. Move ${appPath} aside before copying a replacement build.`
  );
  process.exit(0);
}

if (existsSync(appPath)) {
  console.warn(
    `[synara-desktop] Local install note: move ${appPath} aside before copying a smoke-test build, otherwise cp -R can nest the new Synara.app inside the old bundle.`
  );
}
