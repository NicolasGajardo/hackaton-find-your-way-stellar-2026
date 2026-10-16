import { BLEND_USDC_FAUCET_URL, SHARE_DECIMALS, USDC_DECIMALS } from '../config'
import type { useDeposit } from '../hooks/useDeposit'
import { formatUnits } from '../lib/units'

type Props = {
  address: string | null
  wrongNetwork: boolean
  usdcBalance: bigint | null
  deposit: ReturnType<typeof useDeposit>
}

const PAUSED_NOTICE = 'Deposits are paused. Redemptions are still available.'

export function DepositPanel({ address, wrongNetwork, usdcBalance, deposit }: Props) {
  if (!address) {
    return (
      <section className="panel panel-muted" aria-labelledby="deposit-title">
        <h2 id="deposit-title">Deposit</h2>
        <p className="muted">Connect your wallet to deposit USDC.</p>
      </section>
    )
  }

  const {
    input,
    setInput,
    setMax,
    assets,
    preview,
    exceedsBalance,
    exceedsCap,
    paused,
    needsBlendUsdc,
    status,
    submit,
    canSubmit,
  } = deposit
  const signing = status.kind === 'signing'
  const formLocked = signing || paused

  return (
    <section className="panel" aria-labelledby="deposit-title">
      <h2 id="deposit-title">Deposit</h2>
      {paused && (
        <p className="alert alert-warning" role="status">
          {PAUSED_NOTICE}
        </p>
      )}
      {needsBlendUsdc && (
        <p className="alert alert-warning">
          <a href={BLEND_USDC_FAUCET_URL} target="_blank" rel="noreferrer">
            You need Blend test USDC
          </a>
        </p>
      )}
      <form
        className="form"
        onSubmit={(e) => {
          e.preventDefault()
          void submit()
        }}
      >
        <label className="field-label" htmlFor="deposit-amount">
          Amount in USDC
        </label>
        <div className="field">
          <input
            id="deposit-amount"
            className="input"
            inputMode="decimal"
            autoComplete="off"
            placeholder="0.00"
            value={input}
            disabled={formLocked}
            onChange={(e) => setInput(e.target.value)}
          />
          <button
            type="button"
            className="btn btn-ghost"
            onClick={setMax}
            disabled={formLocked || usdcBalance === null || usdcBalance === 0n}
          >
            Max
          </button>
        </div>

        {usdcBalance !== null && (
          <p className="muted field-hint">
            Available: {formatUnits(usdcBalance, USDC_DECIMALS)} USDC
          </p>
        )}
        {input.trim() !== '' && assets === null && (
          <p className="field-error">Enter a valid amount greater than zero.</p>
        )}
        {exceedsBalance && <p className="field-error">You don't have that much USDC in your wallet.</p>}
        {exceedsCap && !exceedsBalance && (
          <p className="field-error">This deposit exceeds the fund's current cap.</p>
        )}
        {assets !== null && !exceedsBalance && !exceedsCap && preview !== null && (
          <p className="muted field-hint">
            You'll receive ≈ {formatUnits(preview, SHARE_DECIMALS, { maxFractionDigits: 7 })} shares
          </p>
        )}

        <button type="submit" className="btn btn-primary" disabled={!canSubmit || wrongNetwork}>
          {signing ? 'Signing in your wallet…' : 'Deposit'}
        </button>
      </form>

      {wrongNetwork && (
        <p className="alert alert-warning">Switch your wallet to testnet to deposit.</p>
      )}
      {status.kind === 'success' && (
        <p className="alert alert-success" role="status">
          Deposit complete. You received{' '}
          {formatUnits(status.shares, SHARE_DECIMALS, { maxFractionDigits: 7 })} shares.
        </p>
      )}
      {status.kind === 'error' && (
        <p className="alert alert-error" role="alert">
          {status.message}
        </p>
      )}
    </section>
  )
}
