import fs from 'fs';
import path from 'path';
import { execSync } from 'child_process';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const version = process.argv[2];

if (!version) {
  console.error('Version argument missing');
  process.exit(1);
}

const root = path.resolve(__dirname, '..');
const newVersionTag = `v${version}`;

// Update package.json + package-lock.json safely. Re-running a release bump to
// adjust iOS build metadata must remain idempotent.
const currentPackageVersion = JSON.parse(
  fs.readFileSync(path.join(root, 'package.json'), 'utf8')
).version;
if (currentPackageVersion !== version) {
  execSync(`npm version ${version} --no-git-tag-version`, {
    cwd: root,
    stdio: 'inherit',
  });
}

console.log(`Updated package.json and package-lock.json → ${version}`);

// Update UI version references
const files = [
  'src/app/features/settings/about/About.tsx',
  'src/app/pages/auth/AuthFooter.tsx',
  'src/app/pages/client/WelcomePage.tsx',
];

files.forEach((filePath) => {
  const absPath = path.join(root, filePath);

  if (!fs.existsSync(absPath)) {
    console.warn(`File not found: ${filePath}`);
    return;
  }

  const content = fs.readFileSync(absPath, 'utf8');
  const updated = content.replace(/v\d+\.\d+\.\d+/g, newVersionTag);

  fs.writeFileSync(absPath, updated);

  console.log(`Updated ${filePath} → ${newVersionTag}`);
});
