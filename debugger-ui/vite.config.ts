import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { execSync } from 'node:child_process';

function resolveVersion(): string {
  try {
    return execSync('git describe', { stdio: 'pipe' }).toString().trim() || "unknown";
  } catch {
    return "unknown";
  }
}

const appVersion = resolveVersion();

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  define: {
    __APP_VERSION__: JSON.stringify(appVersion),
  },
});
