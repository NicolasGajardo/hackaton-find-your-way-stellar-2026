# Fondex App

Interfaz web del fondo mutuo USDC de Fondex en **Stellar testnet**: conectar wallet, ver el estado del fondo y tu posición, aportar, rescatar y rebalancear.

## Requisitos

- Node.js 20+ y npm
- [Freighter](https://www.freighter.app/) (u otra wallet compatible con [Stellar Wallets Kit](https://stellarwalletskit.dev/)) en testnet

## Instalación

```bash
cd app
npm install
```

Los bindings del contrato viven en `packages/fondex-vault`. Si cambias el contrato en testnet, regenera:

```bash
stellar contract bindings typescript \
  --network testnet \
  --contract-id CAEGDXJP6KXDLZIS4KOLMVM6IBGDZZDRFQTHCDXK6CZNNHJOVNT6K2ZS \
  --output-dir packages/fondex-vault \
  --overwrite
npm install
```

(`postinstall` aplica `scripts/patch-vault-bindings.mjs`; sin él, `npm run dev` puede fallar por re-exports del SDK en los bindings.)

## Desarrollo

```bash
npm run dev
```

Abre la URL que muestra Vite (por defecto `http://localhost:5173`).

## Scripts

| Comando         | Descripción              |
|-----------------|--------------------------|
| `npm run dev`   | Servidor de desarrollo   |
| `npm run build` | Build de producción      |
| `npm run lint`  | Oxlint                   |
| `npm test`      | Tests unitarios (Vitest) |

## Configuración

Crea `.env.local` (está en `.gitignore`):

```bash
VITE_VAULT_CONTRACT_ID=CAEGDXJP6KXDLZIS4KOLMVM6IBGDZZDRFQTHCDXK6CZNNHJOVNT6K2ZS
VITE_USDC_CONTRACT_ID=CAQCFVLOBK5GIULPNZRGATJJMIZL5BSP7X5YJVMGCPTUEPFM4AVSRCJU
```

Esos valores son también los predeterminados en `src/config.ts`. Opcionales:

- `VITE_STELLAR_RPC_URL` — RPC Soroban (default: testnet público)
- `VITE_HORIZON_URL` — Horizon testnet

El USDC es el de Blend: código `USDC`, emisor `GATALTGTWIOT6BUDBCZM3Q4OQ4BO2COLOAZ7IYSKPLC2PMSOPPGF5V56`.

## El fondo en testnet

Bóveda: `CAEGDXJP6KXDLZIS4KOLMVM6IBGDZZDRFQTHCDXK6CZNNHJOVNT6K2ZS`.

Recibe aportes en USDC y deja una reserva. Cuando alguien llama **Rebalancear**, invierte el resto en la estrategia Blend de DeFindex (`CALLOM5I7XLQPPOPQMYAHUWW4N7O3JKT42KQ4ASEEVBXDJQNJOALFSUY`): aportar no invierte solo. Los rescates se pagan primero con la reserva; si no alcanza, la misma operación saca la diferencia de Blend.

### USDC de prueba de Blend

Este USDC **no** es el USDC de Circle en testnet. Para conseguirlo:

1. Configura la wallet en **Testnet** y fondea la cuenta con [Friendbot](https://lab.stellar.org/account/fund) si todavía no existe.
2. Abre https://testnet.blend.capital y usa el botón del faucet que aparece arriba de la página. Entrega USDC de prueba y abre la trustline. Solo aparece si a tu cuenta le falta alguna trustline de los activos de Blend.

### Probar el fondo

1. `npm run dev` y abre la app.
2. Sin conectar la wallet, **El fondo** muestra el total, las cuotas, el valor cuota, la rentabilidad desde el inicio (la cuota partió en 1 USDC) y la composición.
3. **Conectar wallet** → elige Freighter y autoriza.
4. Si tu saldo de USDC de Blend es 0, **Aportar** muestra un enlace al faucet.
5. Escribe un monto (o **Máx**) y toca **Aportar**. El máximo lo marca `max_deposit`.
6. Si la composición se desvió más de 2 puntos del objetivo, **Rebalancear** se resalta. Cualquiera puede llamarlo; solo pagas la comisión de red.
7. En **Rescatar**, un monto usa `withdraw`, y **Máx** quema todas tus cuotas con `redeem`. El texto bajo el formulario indica cuánto sale de la reserva.
