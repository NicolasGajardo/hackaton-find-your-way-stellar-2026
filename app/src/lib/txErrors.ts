/** Error codes from the vault and the USDC token. */
const CONTRACT_ERRORS: Record<number, string> = {
  1: 'The amount must be greater than zero.',
  2: 'This deposit is too small to receive any shares.',
  3: 'This redemption is too small.',
  4: 'Blend does not have enough liquidity to pay this redemption right now. Try a smaller amount or try again later.',
  5: 'This deposit exceeds the fund\'s current cap.',
  6: 'One of the fund\'s investments is not responding; deposits are paused for safety. Redemptions are still available.',
  10: 'You don\'t have enough USDC for this deposit.',
  13: 'Your account has no USDC trustline. Add it in your wallet.',
  100: 'You don\'t have enough shares for this operation.',
  1000: 'The fund is paused: deposits and rebalancing are stopped. Redemptions are still available.',
  1500: 'The amount is too large.',
}

/** The active wallet account is no longer the one shown in the app. */
export const WALLET_ACCOUNT_CHANGED = 'WALLET_ACCOUNT_CHANGED'

/**
 * Turns a signing / simulation / submission error into a readable message.
 * Soroban reports contract errors as `Error(Contract, #N)`.
 * The vault's admin codes 10 to 20 are not used from the app and fall back to
 * the generic message, except 10 and 13, which come from the USDC token.
 */
export function describeTxError(err: unknown): string {
  // The kit rejects with `{ code, message }` objects that are not Error instances.
  const message =
    err instanceof Error
      ? err.message
      : typeof err === 'object' && err !== null && 'message' in err
        ? String(err.message)
        : String(err)

  if (message === WALLET_ACCOUNT_CHANGED) {
    return 'You switched accounts in your wallet. Disconnect and connect again.'
  }
  if (/not (connected|allowed)|request access/i.test(message)) {
    return 'Your wallet has not authorized this site. Disconnect and connect your wallet again.'
  }

  const contractCode = /Error\(Contract, #(\d+)\)/.exec(message)
  if (contractCode) {
    const code = Number(contractCode[1])
    return CONTRACT_ERRORS[code] ?? `The contract rejected the operation (code ${code}).`
  }

  if (/reject|declin|denied|cancel/i.test(message)) {
    return 'You cancelled the signature in your wallet.'
  }
  if (/trustline/i.test(message)) {
    return CONTRACT_ERRORS[13]
  }
  if (/Account not found|Resource missing/i.test(message)) {
    return 'Your account does not exist on testnet. Fund it with Friendbot first.'
  }

  return message || 'The transaction failed.'
}
