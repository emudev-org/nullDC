import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';
import { execSync } from 'node:child_process';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const pkg = JSON.parse(readFileSync(resolve(__dirname, 'package.json'), 'utf8'));
const BASE_VERSION = '2.0.0-pre';

function resolveVersion(): string {
  if (pkg.version && pkg.version !== '__APP_VERSION__') {
    return pkg.version;
  }
  try {
    const rev = execSync('git rev-parse --short HEAD', { stdio: 'pipe' }).toString().trim();
    if (rev) {
      return `${BASE_VERSION}+${rev}`;
    }
  } catch {
    // ignore
  }
  return BASE_VERSION;
}

const appVersion = resolveVersion();

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  define: {
    __APP_VERSION__: JSON.stringify(appVersion),
  },
});
