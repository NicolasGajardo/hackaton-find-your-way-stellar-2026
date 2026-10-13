/**
 * Tras regenerar bindings con `stellar contract bindings typescript`, Vite 8
 * falla con `export * from "@stellar/stellar-sdk"`. Este script elimina esos re-exports.
 */
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const indexPath = path.join(
  path.dirname(fileURLToPath(import.meta.url)),
  '../packages/fondex-vault/src/index.ts',
)

let source = fs.readFileSync(indexPath, 'utf8')
source = source.replace(
  /import \{ Address \} from "@stellar\/stellar-sdk";\n/,
  '',
)
source = source.replace(
  /export \* from "@stellar\/stellar-sdk";\nexport \* as contract from "@stellar\/stellar-sdk\/contract";\nexport \* as rpc from "@stellar\/stellar-sdk\/rpc";\n\n/,
  '\n',
)
fs.writeFileSync(indexPath, source)
console.log('Patched fondex-vault bindings for Vite compatibility.')
