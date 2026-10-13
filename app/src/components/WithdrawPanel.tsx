import { SHARE_DECIMALS, USDC_DECIMALS } from '../config'
import type { useWithdraw } from '../hooks/useWithdraw'
import { formatUnits } from '../lib/units'

type Props = {
  address: string | null
  wrongNetwork: boolean
  idleAssets: bigint | null
  withdraw: ReturnType<typeof useWithdraw>
}

export function WithdrawPanel({ address, wrongNetwork, idleAssets, withdraw }: Props) {
  if (!address) {
    return (
      <section className="panel panel-muted" aria-labelledby="withdraw-title">
        <h2 id="withdraw-title">Rescatar</h2>
        <p className="muted">Conecta tu wallet para rescatar USDC.</p>
      </section>
    )
  }

  const {
    input,
    setInput,
    setMax,
    assets,
    maxAssets,
    redeemAll,
    sharesToBurn,
    exceedsPosition,
    status,
    submit,
    canSubmit,
  } = withdraw
  const signing = status.kind === 'signing'

  return (
    <section className="panel" aria-labelledby="withdraw-title">
      <h2 id="withdraw-title">Rescatar</h2>
      <form
        className="form"
        onSubmit={(e) => {
          e.preventDefault()
          void submit()
        }}
      >
        <label className="field-label" htmlFor="withdraw-amount">
          Monto en USDC
        </label>
        <div className="field">
          <input
            id="withdraw-amount"
            className="input"
            inputMode="decimal"
            autoComplete="off"
            placeholder="0.00"
            value={input}
            disabled={signing}
            onChange={(e) => setInput(e.target.value)}
          />
          <button
            type="button"
            className="btn btn-ghost"
            onClick={setMax}
            disabled={signing || maxAssets === null || maxAssets === 0n}
          >
            Máx
          </button>
        </div>

        {maxAssets !== null && (
          <p className="muted field-hint">
            Puedes rescatar hasta {formatUnits(maxAssets, USDC_DECIMALS)} USDC
          </p>
        )}
        {input.trim() !== '' && assets === null && (
          <p className="field-error">Escribe un monto válido mayor que cero.</p>
        )}
        {exceedsPosition && (
          <p className="field-error">Tu inversión no alcanza para rescatar ese monto.</p>
        )}
        {assets !== null && !exceedsPosition && sharesToBurn !== null && (
          <p className="muted field-hint">
            {redeemAll ? 'Se usarán todas tus cuotas' : 'Se usarán ≈'}{' '}
            {formatUnits(sharesToBurn, SHARE_DECIMALS, { maxFractionDigits: 7 })} cuotas
          </p>
        )}

        <button type="submit" className="btn btn-primary" disabled={!canSubmit || wrongNetwork}>
          {signing ? 'Firmando en tu wallet…' : redeemAll ? 'Rescatar todo' : 'Rescatar'}
        </button>
      </form>

      {idleAssets !== null && (
        <p className="muted field-hint">
          Hasta {formatUnits(idleAssets, USDC_DECIMALS)} USDC se pagan desde la reserva; si pides
          más, el fondo saca la diferencia de Blend en la misma operación.
        </p>
      )}

      {wrongNetwork && (
        <p className="alert alert-warning">Cambia tu wallet a testnet para rescatar.</p>
      )}
      {status.kind === 'success' && (
        <p className="alert alert-success" role="status">
          Rescate listo. Recibiste {formatUnits(status.usdc, USDC_DECIMALS)} USDC en tu wallet.
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
