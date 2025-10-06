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
const useBroadcast = process.env.VITE_USE_BROADCAST === 'true';

// https://vite.dev/config/
export default defineConfig({
  base: './',
  plugins: [react()],
  define: {
    __APP_VERSION__: JSON.stringify(appVersion),
    __USE_BROADCAST__: JSON.stringify(useBroadcast),
  },
});
