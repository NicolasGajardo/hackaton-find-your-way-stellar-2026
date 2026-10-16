import path from 'node:path'
import { fileURLToPath } from 'node:url'
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

const rootDir = path.dirname(fileURLToPath(import.meta.url))

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  define: {
    global: 'globalThis',
  },
  resolve: {
    alias: {
      buffer: 'buffer',
      // Avoids consuming dist/, whose re-exports Rolldown can't interop with the SDK.
      'fondex-vault': path.resolve(rootDir, 'packages/fondex-vault/src/index.ts'),
    },
  },
  optimizeDeps: {
    include: [
      'buffer',
      '@stellar/stellar-sdk',
      '@stellar/stellar-sdk/contract',
      '@stellar/stellar-sdk/rpc',
      '@creit.tech/stellar-wallets-kit/sdk',
      '@creit.tech/stellar-wallets-kit/modules/utils',
      '@creit.tech/stellar-wallets-kit/types',
    ],
  },
  ssr: {
    noExternal: [
      '@creit.tech/stellar-wallets-kit',
      '@stellar/freighter-api',
      '@lobstrco/signer-extension-api',
    ],
  },
})
