import { copyFileSync, existsSync, mkdirSync, rmSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const projectRoot = join(__dirname, '..');

const targetDir = join(projectRoot, 'src-tauri', 'target', 'release');
const releaseDir = join(projectRoot, 'release');
if (!existsSync(releaseDir)) {
  mkdirSync(releaseDir, { recursive: true });
}

const exeName = 'StatsWidget.exe';
const builtExe = join(targetDir, exeName);
if (existsSync(builtExe)) {
  copyFileSync(builtExe, join(releaseDir, exeName));
  console.log(`[OK] Portable EXE : release/${exeName}`);
} else {
  console.error(`[!!] EXE not found at ${builtExe}`);
  process.exit(1);
}

if (existsSync(targetDir)) {
  rmSync(targetDir, { recursive: true, force: true });
}
