import { abbreviateAddress } from '../lib/units'

type Props = {
  address: string | null
  connecting: boolean
  wrongNetwork: boolean
  networkLabel: string | null
  onConnect: () => void
  onDisconnect: () => void
}

export function WalletButton({
  address,
  connecting,
  wrongNetwork,
  networkLabel,
  onConnect,
  onDisconnect,
}: Props) {
  if (!address) {
    return (
      <button type="button" className="btn btn-primary" onClick={onConnect} disabled={connecting}>
        {connecting ? 'Conectando…' : 'Conectar wallet'}
      </button>
    )
  }

  return (
    <div className="wallet-bar">
      <span className="wallet-address" title={address}>
        {abbreviateAddress(address)}
      </span>
      <button type="button" className="btn btn-ghost" onClick={onDisconnect}>
        Desconectar
      </button>
      {wrongNetwork && (
        <p className="alert alert-warning" role="alert">
          Tu wallet no está en testnet
          {networkLabel ? ` (${networkLabel})` : ''}. Cambia a testnet para usar Fondex.
        </p>
      )}
    </div>
  )
}
