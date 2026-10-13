import './App.css'
import { DepositPanel } from './components/DepositPanel'
import { FundPanel } from './components/FundPanel'
import { InvestmentPanel } from './components/InvestmentPanel'
import { WalletButton } from './components/WalletButton'
import { WithdrawPanel } from './components/WithdrawPanel'
import { useDeposit } from './hooks/useDeposit'
import { useRebalance } from './hooks/useRebalance'
import { useVaultPanels } from './hooks/useVaultPanels'
import { useWallet } from './hooks/useWallet'
import { useWithdraw } from './hooks/useWithdraw'

function App() {
  const wallet = useWallet()
  const panels = useVaultPanels(wallet.address)
  const paused = panels.fund?.paused === true
  const deposit = useDeposit(
    wallet.address,
    panels.usdcBalance,
    panels.maxDeposit,
    paused,
    panels.refreshAll,
  )
  const withdraw = useWithdraw(
    wallet.address,
    panels.userVault,
    panels.maxWithdraw,
    panels.refreshAll,
  )
  const rebalance = useRebalance(
    wallet.address,
    paused,
    wallet.wrongNetwork,
    panels.refreshAll,
  )

  return (
    <div className="app">
      <header className="header">
        <div>
          <p className="eyebrow">Fondex · testnet</p>
          <h1>Fondo mutuo USDC</h1>
          <p className="lede">
            Fondex es un fondo mutuo descentralizado. Invierte en préstamos de USDC en Blend y
            guarda una reserva para pagar rescates. Puedes rescatar cuando quieras.
          </p>
        </div>
        <WalletButton
          address={wallet.address}
          connecting={wallet.connecting}
          wrongNetwork={wallet.wrongNetwork}
          networkLabel={wallet.networkLabel}
          onConnect={() => void wallet.connect()}
          onDisconnect={() => void wallet.disconnect()}
        />
      </header>

      <main className="panels">
        <FundPanel
          fund={panels.fund}
          loading={panels.fundState === 'loading'}
          error={panels.fundState === 'error' ? panels.error : null}
          address={wallet.address}
          wrongNetwork={wallet.wrongNetwork}
          rebalance={rebalance}
          onRetry={() => void panels.refreshAll()}
        />
        <InvestmentPanel
          address={wallet.address}
          userVault={panels.userVault}
          usdcBalance={panels.usdcBalance}
          loading={panels.userState === 'loading'}
        />
        <DepositPanel
          address={wallet.address}
          wrongNetwork={wallet.wrongNetwork}
          usdcBalance={panels.usdcBalance}
          deposit={deposit}
        />
        <WithdrawPanel
          address={wallet.address}
          wrongNetwork={wallet.wrongNetwork}
          idleAssets={panels.fund?.idleAssets ?? null}
          withdraw={withdraw}
        />
      </main>

      <footer className="footer">
        <button type="button" className="btn btn-ghost" onClick={() => void panels.refreshAll()}>
          Actualizar datos
        </button>
      </footer>
    </div>
  )
}

export default App
