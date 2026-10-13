# Fondex — fondo mutuo descentralizado en Stellar

> **English:** Fondex is a decentralized mutual fund on Stellar. Users deposit USDC, receive fund shares (`fxFUND`) and can redeem at any time. The fund lends 90% of its USDC on [Blend](https://www.blend.capital/) through an audited [DeFindex](https://www.defindex.io/) strategy and keeps 10% as a liquidity reserve. Soroban (Rust) contracts + React web app. **Testnet prototype, not audited.**

Fondex es un **fondo mutuo descentralizado**: cualquier persona aporta dólares digitales (USDC), recibe cuotas del fondo y puede rescatar cuando quiera. Las reglas las impone un contrato público en Stellar, no un intermediario.

- El fondo invierte el **90%** en préstamos de USDC en **Blend**, a través de una estrategia de **DeFindex** ya desplegada y auditada.
- Guarda un **10% de reserva** para pagar rescates al instante. Si un rescate supera la reserva, el contrato saca la diferencia de Blend en la misma operación.
- Los intereses suben el **valor cuota**.

> ⚠️ **Prototipo en testnet.** Los contratos no están auditados. No usar con dinero real.

## Demo en testnet

| Qué | Dirección |
|---|---|
| Bóveda FondexVault | [`CAEGDXJP6KXDLZIS4KOLMVM6IBGDZZDRFQTHCDXK6CZNNHJOVNT6K2ZS`](https://stellar.expert/explorer/testnet/contract/CAEGDXJP6KXDLZIS4KOLMVM6IBGDZZDRFQTHCDXK6CZNNHJOVNT6K2ZS) |
| USDC de prueba de Blend (SAC) | `CAQCFVLOBK5GIULPNZRGATJJMIZL5BSP7X5YJVMGCPTUEPFM4AVSRCJU` |
| Estrategia Blend USDC de DeFindex | `CALLOM5I7XLQPPOPQMYAHUWW4N7O3JKT42KQ4ASEEVBXDJQNJOALFSUY` |

## Estructura

```
contracts/   Contrato Soroban (Rust) de la bóveda, tests e invariantes, scripts de testnet
app/         App web (Vite + React + TypeScript): conectar wallet, aportar, rescatar, ver la composición y rebalancear
docs/        Visión, modelo de negocio y diseño técnico
```

## Probar la app

Requisitos: Node.js 22, y [Freighter](https://www.freighter.app/) en **Test Net**.

```bash
cd app
npm install
npm run dev
```

Abre `http://localhost:5173` y conecta Freighter.

Para aportar necesitas **USDC de prueba de Blend** (no es el mismo USDC de otros faucets):
1. Entra a [testnet.blend.capital](https://testnet.blend.capital) con Freighter en testnet.
2. Usa el botón del faucet que aparece arriba de la página. Solo se muestra si a tu cuenta le falta alguna trustline de los activos de Blend.

Más detalles en [`app/README.md`](app/README.md).

## Contratos

Requisitos: Rust, [Stellar CLI](https://developers.stellar.org/docs/tools/cli) 26 y `make`.

```bash
cd contracts
make ci          # formato, clippy, tests (incluye invariantes con secuencias aleatorias) y build WASM
```

- Despliegue en testnet con un comando: `./scripts/deploy-testnet.sh`.
- Prueba de punta a punta contra Blend: `./scripts/e2e-testnet.sh <VAULT_ID>`.

Más detalles en [`contracts/README.md`](contracts/README.md).

### Garantías del contrato

- **Fondex opera, el usuario es dueño:** el USDC solo puede ir a estrategias aprobadas o a quien rescata sus cuotas. Ningún rol puede transferirlo a otra dirección.
- **Rescate siempre disponible:** la pausa detiene aportes y rebalanceo, nunca los rescates.
- **Plazo de espera público** para agregar estrategias, máximo 4 estrategias y reserva mínima del 5%.
- **Tope de depósitos** configurable, para limitar el riesgo al inicio.
- **No actualizable:** nadie puede cambiar las reglas después del despliegue.
- **Confiable:** si una estrategia deja de responder, los aportes se bloquean y los rescates siguen funcionando.

## Documentación

- [Resumen: idea y modelo de negocio](docs/00-resumen.md)
- [Visión y decisiones](docs/01-vision.md)
- [Diseño técnico del contrato](docs/02-diseno-tecnico.md)

## Estado y próximos pasos

- ✅ **Hecho (testnet):** fondo mutuo USDC con inversión en Blend vía DeFindex, reserva, rebalanceo, administración con plazo de espera, pausa, tope de depósitos y app web.
- **Próximo:** keeper (rebalanceo automático), comisiones, auditoría.
- **Futuro:** fondos de otros activos (wBTC, wETH, XLM…) y RWA con oráculos.

Este repositorio es una copia pública del código para demostración.

## Licencia

[Apache-2.0](LICENSE).
