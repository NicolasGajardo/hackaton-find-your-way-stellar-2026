import { BLEND_USDC_FAUCET_URL, SHARE_DECIMALS, USDC_DECIMALS } from '../config'
import type { useDeposit } from '../hooks/useDeposit'
import { formatUnits } from '../lib/units'

type Props = {
  address: string | null
  wrongNetwork: boolean
  usdcBalance: bigint | null
  deposit: ReturnType<typeof useDeposit>
}

const PAUSED_NOTICE = 'Los aportes están pausados. Los rescates siguen disponibles.'

export function DepositPanel({ address, wrongNetwork, usdcBalance, deposit }: Props) {
  if (!address) {
    return (
      <section className="panel panel-muted" aria-labelledby="deposit-title">
        <h2 id="deposit-title">Aportar</h2>
        <p className="muted">Conecta tu wallet para aportar USDC.</p>
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
      <h2 id="deposit-title">Aportar</h2>
      {paused && (
        <p className="alert alert-warning" role="status">
          {PAUSED_NOTICE}
        </p>
      )}
      {needsBlendUsdc && (
        <p className="alert alert-warning">
          <a href={BLEND_USDC_FAUCET_URL} target="_blank" rel="noreferrer">
            Necesitas USDC de prueba de Blend
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
          Monto en USDC
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
            Máx
          </button>
        </div>

        {usdcBalance !== null && (
          <p className="muted field-hint">
            Disponible: {formatUnits(usdcBalance, USDC_DECIMALS)} USDC
          </p>
        )}
        {input.trim() !== '' && assets === null && (
          <p className="field-error">Escribe un monto válido mayor que cero.</p>
        )}
        {exceedsBalance && <p className="field-error">No tienes tanto USDC en tu wallet.</p>}
        {exceedsCap && !exceedsBalance && (
          <p className="field-error">El aporte supera el tope actual del fondo.</p>
        )}
        {assets !== null && !exceedsBalance && !exceedsCap && preview !== null && (
          <p className="muted field-hint">
            Recibirás ≈ {formatUnits(preview, SHARE_DECIMALS, { maxFractionDigits: 7 })} cuotas
          </p>
        )}

        <button type="submit" className="btn btn-primary" disabled={!canSubmit || wrongNetwork}>
          {signing ? 'Firmando en tu wallet…' : 'Aportar'}
        </button>
      </form>

      {wrongNetwork && (
        <p className="alert alert-warning">Cambia tu wallet a testnet para aportar.</p>
      )}
      {status.kind === 'success' && (
        <p className="alert alert-success" role="status">
          Aporte listo. Recibiste{' '}
          {formatUnits(status.shares, SHARE_DECIMALS, { maxFractionDigits: 7 })} cuotas.
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
