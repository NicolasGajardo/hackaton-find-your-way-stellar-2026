/** Red y contratos — testnet únicamente. */
export const NETWORK_PASSPHRASE = 'Test SDF Network ; September 2015'

export const RPC_URL =
  import.meta.env.VITE_STELLAR_RPC_URL ?? 'https://soroban-testnet.stellar.org'

export const HORIZON_URL =
  import.meta.env.VITE_HORIZON_URL ?? 'https://horizon-testnet.stellar.org'

export const VAULT_CONTRACT_ID =
  import.meta.env.VITE_VAULT_CONTRACT_ID ??
  'CAEGDXJP6KXDLZIS4KOLMVM6IBGDZZDRFQTHCDXK6CZNNHJOVNT6K2ZS'

/** SAC del USDC de prueba de Blend en testnet. */
export const USDC_CONTRACT_ID =
  import.meta.env.VITE_USDC_CONTRACT_ID ??
  'CAQCFVLOBK5GIULPNZRGATJJMIZL5BSP7X5YJVMGCPTUEPFM4AVSRCJU'

export const USDC_ASSET_CODE = 'USDC'
export const USDC_ISSUER =
  'GATALTGTWIOT6BUDBCZM3Q4OQ4BO2COLOAZ7IYSKPLC2PMSOPPGF5V56'

/** Faucet de USDC de Blend. También abre la trustline si falta. */
export const BLEND_USDC_FAUCET_URL = 'https://testnet.blend.capital'

export const USDC_DECIMALS = 7
export const SHARE_DECIMALS = 17

/** 1 cuota completa en unidades mínimas (10^17). */
export const ONE_SHARE = 10n ** 17n

/** Nombres visibles de las estrategias. La clave es la dirección del contrato. */
export const STRATEGY_NAMES: Record<string, string> = {
  CALLOM5I7XLQPPOPQMYAHUWW4N7O3JKT42KQ4ASEEVBXDJQNJOALFSUY:
    'Blend · préstamos de USDC (vía DeFindex)',
}
