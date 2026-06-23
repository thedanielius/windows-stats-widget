import { copyFileSync, existsSync, mkdirSync, rmSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const projectRoot = join(__dirname, '..');

const targetDir = join(projectRoot, 'src-tauri', 'target', 'release');
if (existsSync(targetDir)) {
  console.log('Cleaning previous build...');
  rmSync(targetDir, { recursive: true, force: true });
}

const releaseDir = join(projectRoot, 'release');
if (!existsSync(releaseDir)) {
  mkdirSync(releaseDir, { recursive: true });
}

const exeName = 'StatsWidget.exe';
console.log('Copying to release/...');
// After tauri build, the exe is at this path
const builtExe = join(targetDir, exeName);
if (existsSync(builtExe)) {
  copyFileSync(builtExe, join(releaseDir, exeName));
  console.log(`[OK] Portable EXE : release/${exeName}`);
} else {
  console.error(`[!!] EXE not found at ${builtExe}`);
  process.exit(1);
}
