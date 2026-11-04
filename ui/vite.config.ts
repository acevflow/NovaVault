/// <reference types="vitest/config" />
import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';

// Settings follow the Tauri 2 + Vite guide: a fixed port so the Tauri shell
// can find the dev server, and no screen clearing so Rust errors stay visible.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    // Never expose the dev server beyond this machine.
    host: 'localhost',
  },
  envPrefix: ['VITE_', 'TAURI_ENV_'],
  build: {
    // The webviews Tauri targets all support modern JavaScript.
    target: 'es2022',
    // Release bundles ship without source maps.
    sourcemap: false,
    // The password strength dictionaries are one large, lazily loaded chunk
    // (about 1.2 MB). It is read from disk by the webview, never downloaded.
    chunkSizeWarningLimit: 1500,
  },
  test: {
    environment: 'jsdom',
    setupFiles: ['./src/test/setup.ts'],
    restoreMocks: true,
  },
});
