import { describe, expect, it } from 'vitest'
import { describeTxError } from './txErrors'

describe('describeTxError', () => {
  it('translates USDC token errors', () => {
    const err = new Error('HostError: Error(Contract, #10)\nEvent log...')
    expect(describeTxError(err)).toBe("You don't have enough USDC for this deposit.")
  })

  it('translates the vault running out of liquidity', () => {
    expect(describeTxError(new Error('Error(Contract, #4)'))).toBe(
      'Blend does not have enough liquidity to pay this redemption right now. Try a smaller amount or try again later.',
    )
  })

  it('translates the fund being paused', () => {
    expect(describeTxError(new Error('Error(Contract, #1000)'))).toBe(
      'The fund is paused: deposits and rebalancing are stopped. Redemptions are still available.',
    )
  })

  it('falls back to the generic message for codes the vault does not use', () => {
    expect(describeTxError(new Error('Error(Contract, #405)'))).toBe(
      'The contract rejected the operation (code 405).',
    )
  })

  it('shows the code when the contract error is unknown', () => {
    expect(describeTxError(new Error('Error(Contract, #14)'))).toBe(
      'The contract rejected the operation (code 14).',
    )
  })

  it('detects that the user cancelled the signature', () => {
    expect(describeTxError(new Error('User declined access'))).toBe(
      'You cancelled the signature in your wallet.',
    )
  })

  it('reads kit errors that come as a { code, message } object', () => {
    const err = { code: -3, message: 'The site is not connected to Freighter' }
    expect(describeTxError(err)).toBe(
      'Your wallet has not authorized this site. Disconnect and connect your wallet again.',
    )
  })
})
