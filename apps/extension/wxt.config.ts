import { defineConfig } from 'wxt';

// Least privilege: no host permissions. Filling uses activeTab, granted only when the user opens the
// popup or presses the shortcut on a page. See docs/ARCHITECTURE.md and docs/THREAT_MODEL.md.
export default defineConfig({
  modules: ['@wxt-dev/module-svelte'],
  manifest: ({ browser, mode }) => ({
    name: 'Scytale',
    description: 'A free, open-source password manager with no server. Your vault is encrypted on your device.',
    homepage_url: 'https://github.com/abdvlrqhman/Scytale',
    permissions: [
      'storage',
      'alarms',
      'activeTab',
      'scripting',
      'idle',
      ...(browser === 'firefox' ? ['clipboardWrite'] : ['offscreen']),
    ],
    // End-to-end tests drive a local fixture page without a user gesture; release builds never get this.
    ...(mode === 'e2e' ? { host_permissions: ['http://localhost/*', 'http://127.0.0.1/*'] } : {}),
    commands: {
      'fill-login': {
        suggested_key: { default: 'Ctrl+Shift+L', mac: 'Command+Shift+L' },
        description: 'Fill the login for this page',
      },
    },
    content_security_policy: {
      // WASM is the vault core; nothing else may run that is not part of the package.
      extension_pages: "script-src 'self' 'wasm-unsafe-eval'; object-src 'self'",
    },
    action: { default_title: 'Scytale' },
    ...(browser === 'firefox'
      ? {
          browser_specific_settings: {
            gecko: {
              id: 'scytale@abdvlrqhman.github.io',
              // 140 is the first version with data_collection_permissions (and the current ESR).
              strict_min_version: '140.0',
              data_collection_permissions: { required: ['none'] },
            },
            gecko_android: { strict_min_version: '142.0' },
          },
        }
      : {}),
  }),
});
