# Fondex — contratos Soroban

`contracts/fondex_vault` es la bóveda del fondo mutuo: recibe USDC, emite cuotas `fxFUND` (token SEP-41 dentro del mismo contrato), invierte en estrategias externas con `rebalance` y mantiene una reserva para pagar rescates.

Construido con OpenZeppelin Stellar Contracts 0.7.2 (`stellar-tokens`, `stellar-access`, `stellar-contract-utils`) y `soroban-sdk` 26.1. El diseño completo está en [`../docs/02-diseno-tecnico.md`](../docs/02-diseno-tecnico.md).

## Cómo funciona

- **Constructor:** `__constructor(asset, owner, strategy_timelock, deposit_cap)`. Recibe la dirección del USDC, el dueño (Ownable de OpenZeppelin), el plazo de espera en segundos antes de aceptar una estrategia propuesta (testnet: `60`) y el tope de depósitos (`i128::MAX` en testnet).
- **Aportar (`deposit`) no invierte.** Cualquiera puede llamar `rebalance` para mover USDC hacia el `target_bps` de cada estrategia.
- **Rescatar (`withdraw` / `redeem`) funciona siempre**, incluso con el fondo pausado. Paga primero con la reserva y saca lo que falte de las estrategias.
- **Admin (`owner`):**
  - `propose_strategy` → esperar el plazo → `accept_strategy`;
  - `cancel_strategy`, `set_target`, `pause_strategy` / `unpause_strategy`, `remove_strategy`;
  - `set_deposit_cap` y `pause` / `unpause` (general);
  - cambio de dueño en dos pasos: `transfer_ownership` / `accept_ownership`.

  La suma de asignaciones, incluidas las estrategias pausadas, tiene un máximo de 95%: siempre queda al menos 5% de reserva.
- **Tope de depósitos:** `deposit` rechaza montos que harían superar `deposit_cap`. `max_deposit` es `0` si el fondo está pausado o si alguna estrategia no responde.
- **Estrategia que no responde:** si falla la consulta de saldo de una estrategia, `deposit` y `preview_deposit` fallan con `StrategyUnavailable`. Los rescates usan una valoración conservadora (esa estrategia cuenta como 0) y `rebalance` la salta.
- **Redondeo siempre a favor de la bóveda.** Las cuotas tienen 17 decimales (7 del USDC + offset de 10), lo que mitiga el ataque de inflación en una bóveda vacía.

## Compilar y probar

Requisitos: Rust con el target `wasm32v1-none`, [Stellar CLI](https://developers.stellar.org/docs/tools/cli) 26 y `make`.

```bash
make ci     # formato, clippy (-D warnings), tests y build WASM
```

Los tests incluyen una estrategia falsa (`src/test/mock_strategy.rs`) que simula interés, pérdidas, falta de liquidez y redondeo, y tests de invariantes con secuencias aleatorias reproducibles. Hay una versión larga de esos tests que se corre a mano: `cargo test -p fondex-vault -- --ignored`.

WASM: `target/wasm32v1-none/release/fondex_vault.wasm`.

## Desplegar en testnet

Usa el **USDC de prueba de Blend**. La configuración pública (sin claves) está en `scripts/testnet.env`, y cualquier variable se puede sobrescribir con `VAR=…`.

Necesitas la identidad `dev` en `stellar keys`, con USDC de Blend en esa cuenta: se consigue en https://testnet.blend.capital, y el faucet también abre la trustline.

```bash
./scripts/deploy-testnet.sh              # make ci + deploy + estrategia + depósito inicial
./scripts/deploy-testnet.sh --check-only # solo verificaciones previas
./scripts/deploy-testnet.sh --update-app # pregunta antes de escribir ../app/.env.local
```

El script agrega una fila a la tabla de abajo y **no** hace commit.

### Prueba de punta a punta

```bash
./scripts/e2e-testnet.sh <VAULT_ID>
```

1. Usa una identidad aparte, `e2e-user`. La primera vez la crea y la fondea con friendbot, y `dev` le transfiere 100 USDC de Blend.
2. Aporta y llama `rebalance` como ese usuario.
3. Verifica el reparto ~90/10 y espera el interés de Blend.
4. Rescata 50 USDC, que se sacan de la estrategia, y después rescata el resto.

## Despliegues FondexVault (testnet)

| Fecha | Dirección | Nota |
|---|---|---|
| 2026-10-13 | `CAEGDXJP6KXDLZIS4KOLMVM6IBGDZZDRFQTHCDXK6CZNNHJOVNT6K2ZS` | Blend USDC, objetivo 9000 bps, depósito inicial 1 USDC |
