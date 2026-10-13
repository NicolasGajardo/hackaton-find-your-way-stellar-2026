# Fondex — Diseño técnico: fondo mutuo que invierte con estrategias de DeFindex

> Contrato: [`contracts/contracts/fondex_vault`](../contracts/contracts/fondex_vault) · Desplegado en testnet: `CAEGDXJP6KXDLZIS4KOLMVM6IBGDZZDRFQTHCDXK6CZNNHJOVNT6K2ZS`

---

## 1. Qué construimos

La bóveda **FondexVault** convierte a Fondex en un **fondo mutuo descentralizado**: además de recibir USDC y emitir cuotas (`fxFUND`), **invierte el USDC en estrategias líquidas y de bajo riesgo** para que la cuota suba de valor, y mantiene una reserva para pagar rescates de inmediato.

- La bóveda **no habla con Blend directamente**. Deposita en **estrategias de DeFindex ya desplegadas**, y esas estrategias depositan en Blend.
- **Fondex no escribe estrategias propias:** reutiliza estrategias auditadas y ya desplegadas.

```
Usuario ──deposit/withdraw/redeem──► FondexVault ──deposit/withdraw──► Estrategia Blend USDC ──► Pool Blend v2
                                     (cuotas fxFUND,                    (contrato de DeFindex,     (préstamos)
                                      10% USDC en reserva)               ya desplegado)
```

## 2. Decisiones (cerradas)

| Tema | Decisión |
|---|---|
| Estrategias | **Solo estrategias de DeFindex** ya desplegadas, llamadas directamente en la blockchain. Sin API, sin API key, sin servidores de DeFindex. |
| Estrategias propias de Fondex | **No.** Fuera del alcance actual. |
| Estrategia inicial | Blend USDC de DeFindex (testnet `CALLOM5I7XLQPPOPQMYAHUWW4N7O3JKT42KQ4ASEEVBXDJQNJOALFSUY`) |
| USDC (testnet) | USDC de prueba de Blend `CAQCFVLOBK5GIULPNZRGATJJMIZL5BSP7X5YJVMGCPTUEPFM4AVSRCJU` |
| Asignación inicial | 90% en la estrategia · 10% en reserva (sin invertir) para pagar retiros |
| Cuándo se invierte | Con `rebalance()`, que puede llamar cualquiera. Depositar **no** invierte. |
| Retiros | Se pagan con la reserva; si no alcanza, el mismo retiro saca lo que falta de las estrategias |
| Recompensas BLND | Las reinvierte la propia estrategia de DeFindex. Fondex no hace nada. |
| Admin | Un solo dueño (`owner`, módulo Ownable de OpenZeppelin). En mainnet debe ser una cuenta multifirma. |
| Agregar estrategias | Con plazo de espera (timelock) entre proponer y aceptar |
| Pausa | Bloquea `deposit` y `rebalance`. **Nunca** bloquea `withdraw` ni `redeem`. |
| Actualizable (upgrade) | **No**, ni en testnet ni en mainnet: se prueba exactamente el contrato que irá a producción. Para corregir algo en testnet se **redespliega** con un comando (§12.4). |
| Tope de depósitos | `deposit_cap`: valor máximo del fondo, configurable por el admin. Testnet: sin tope. Mainnet: empezar bajo y subirlo con el tiempo. |

### 2.1 Política de inversión (regla del fondo mutuo)

Fondex es un fondo mutuo: **solo invierte en estrategias líquidas y de bajo riesgo**. Una estrategia solo se puede proponer (`propose_strategy`) si cumple **todos** estos criterios:

| # | Criterio | Qué significa | ¿Lo exige el contrato? |
|---|---|---|---|
| 1 | **Mismo activo** | Recibe y devuelve USDC. Sin exposición al precio de otros activos (XLM, BTC, etc.). | Sí: `AssetMismatch` |
| 2 | **Líquida** | En condiciones normales se puede retirar en la misma transacción, sin colas, plazos de bloqueo ni periodos de aviso. | No: revisión del admin |
| 3 | **Sin apalancamiento ni derivados** | Nada de futuros, opciones, préstamos tomados ni ciclos de préstamo recursivo. | No: revisión del admin |
| 4 | **Sin pérdida impermanente** | Nada de pools de liquidez de dos activos. | No: revisión del admin |
| 5 | **Valor sin oráculo externo** | El valor en USDC lo calcula el propio protocolo (p. ej. `b_rate` de Blend), no un precio de mercado de otro activo. | No: revisión del admin |
| 6 | **Probada** | Contratos auditados, desplegados en mainnet con uso real, y preferentemente no actualizables. | No: revisión del admin |

Además, el contrato garantiza siempre:
- **reserva mínima** de `MIN_IDLE_BPS` (5%) sin invertir, para pagar rescates;
- **plazo de espera** público antes de que una estrategia nueva reciba fondos, para que cualquiera pueda rescatar si no está de acuerdo;
- **máximo `MAX_STRATEGIES`** (4) estrategias.

Los criterios 2 a 6 son compromisos públicos de Fondex: se documentan para cada estrategia antes de proponerla (en el README y en la app). La estrategia Blend USDC de DeFindex cumple los seis.

Cualquier estrategia que no cumpla esta política (futuros, apalancamiento, activos volátiles) **no entra en este fondo**. Si algún día se ofrece, será como un fondo distinto, con su propio contrato y perfil de riesgo.

### Fuera del alcance (no implementar)

Estrategias propias · manejo de BLND · rol de gestor · comisiones · oráculos · depósitos en otras monedas · upgrade · llamar a la API de DeFindex.

Oráculos y otros activos quedan fuera **solo por ahora**, para llegar a un prototipo funcional. Son features futuras (ver "Hacia dónde crece" en [00-resumen.md](00-resumen.md)). La estructura del contrato v2 (un activo + estrategias) sirve para fondos de otro activo, pero **no tal cual**: `MIN_STRATEGY_MOVE`, `WITHDRAW_ROUNDING_BUFFER` y `STRATEGY_DUST` son constantes pensadas para USDC (p. ej. el mínimo de 1 unidad sería 1 wBTC). Un fondo de otro activo necesita una versión con esos valores configurables en el constructor.

## 3. Glosario

| Término | Significado |
|---|---|
| **Cuota (`fxFUND`)** | Token que representa tu parte del fondo. Vive dentro del contrato de la bóveda (SEP-41). |
| **Activos (`assets`)** | USDC, en su unidad mínima (7 decimales). |
| **`total_assets`** | Valor total del fondo en USDC = reserva + lo que vale cada estrategia. |
| **Reserva (`idle`)** | USDC que está dentro de la bóveda, sin invertir. |
| **Estrategia** | Contrato externo (de DeFindex) que recibe USDC de la bóveda y lo invierte. |
| **bps** | Puntos base. 10 000 bps = 100%; 9 000 bps = 90%. |
| **`rebalance`** | Mover USDC entre la reserva y las estrategias para acercarse a la asignación objetivo. |

## 4. Estructura del repositorio

```
contracts/
  Cargo.toml                ← workspace
  contracts/fondex_vault/
    Cargo.toml
    src/
      lib.rs                ← contrato: funciones públicas
      storage.rs            ← claves y lectura/escritura de almacenamiento
      math.rs               ← conversión cuotas ↔ activos
      strategy.rs           ← interfaz de estrategia (cliente) + autorización
      errors.rs             ← FondexError
      events.rs             ← eventos
      test/
        mod.rs
        mock_strategy.rs    ← estrategia FALSA solo para tests (no se despliega)
        deposit_withdraw.rs
        strategies.rs
        rebalance.rs
        admin.rs
        unavailable.rs
        invariants.rs
  scripts/
    deploy-testnet.sh       ← despliegue en testnet con un comando
    e2e-testnet.sh          ← prueba de punta a punta en testnet
```

### Dependencias (versiones exactas)

```toml
soroban-sdk = "26.1.0"
stellar-tokens = "=0.7.2"          # token de cuotas (fungible)
stellar-access = "=0.7.2"          # Ownable
stellar-contract-utils = "=0.7.2"  # pausable + math (mul_div_with_rounding)
```

`fondex_vault` **no** usa `stellar_tokens::vault::Vault`: esa bóveda calcula `total_assets` solo con el USDC que tiene dentro (`vault/storage.rs:142`) y no se puede cambiar. Del crate `stellar-tokens` se usa solo el token de cuotas (`fungible::Base`, `FungibleToken`).

## 5. Constantes

```rust
pub const SHARE_NAME: &str = "Fondex Fund Share";
pub const SHARE_SYMBOL: &str = "fxFUND";
pub const DECIMALS_OFFSET: u32 = 10;            // cuotas = 7 + 10 = 17 decimales
pub const BPS: u32 = 10_000;
pub const MIN_IDLE_BPS: u32 = 500;              // la reserva nunca puede quedar bajo 5%
pub const MAX_STRATEGIES: u32 = 4;
pub const MAX_TIMELOCK_SECS: u64 = 604_800;     // 7 días
pub const REBALANCE_TOLERANCE_BPS: u32 = 200;   // no mover si la desviación es < 2% del fondo
pub const MIN_STRATEGY_MOVE: i128 = 10_000_000; // 1 USDC: no se mueven montos más chicos
pub const WITHDRAW_ROUNDING_BUFFER: i128 = 100; // unidades mínimas extra al sacar de una estrategia
pub const STRATEGY_DUST: i128 = 1_000;          // saldo despreciable al quitar una estrategia
```

## 6. Almacenamiento

Todo en almacenamiento `instance`, con `extend_ttl` en cada función.

```rust
#[contracttype]
pub enum DataKey {
    Asset,               // Address — USDC
    Strategies,          // Vec<StrategyInfo>, en orden de registro
    PendingStrategy,     // PendingStrategy (opcional)
    StrategyTimelock,    // u64 — segundos
    DepositCap,          // i128 — valor máximo de total_assets aceptado al depositar (i128::MAX = sin tope)
}

#[contracttype]
pub struct StrategyInfo {
    pub address: Address,
    pub target_bps: u32,  // % objetivo del fondo en esta estrategia
    pub paused: bool,     // pausada = objetivo 0: no recibe depósitos y rebalance la vacía
}

#[contracttype]
pub struct PendingStrategy {
    pub address: Address,
    pub target_bps: u32,
    pub eta: u64,         // timestamp desde el que se puede aceptar
}
```

- La **reserva objetivo no se guarda**: es `BPS − Σ target_bps` (de las estrategias no pausadas).
- Regla siempre válida: `Σ target_bps ≤ BPS − MIN_IDLE_BPS`, sumando **todas** las estrategias, incluidas las pausadas (así despausar nunca rompe la regla).
- El dueño lo guarda `stellar_access::ownable`. La pausa la guarda `stellar_contract_utils::pausable`.

## 7. Errores

```rust
#[contracterror]
#[repr(u32)]
pub enum FondexError {
    AmountMustBePositive = 1,
    ZeroShares = 2,               // el depósito es tan chico que daría 0 cuotas
    ZeroAssets = 3,               // el rescate es tan chico que daría 0 USDC
    InsufficientLiquidity = 4,    // no hay USDC suficiente entre reserva y estrategias para pagar el retiro
    DepositCapExceeded = 5,       // el depósito haría que total_assets supere deposit_cap
    StrategyUnavailable = 6,      // una estrategia no responde su saldo: no se aceptan depósitos (§10.9)
    TooManyStrategies = 10,
    StrategyAlreadyExists = 11,
    StrategyNotFound = 12,
    AssetMismatch = 13,           // estrategia.asset() ≠ USDC de la bóveda
    AllocationTooHigh = 14,       // Σ target_bps > BPS − MIN_IDLE_BPS
    PendingStrategyExists = 15,
    NoPendingStrategy = 16,
    TimelockNotExpired = 17,
    StrategyNotEmpty = 18,        // al quitar: queda más que STRATEGY_DUST
    InvalidTimelock = 19,         // constructor: timelock > MAX_TIMELOCK_SECS
    InvalidDepositCap = 20,       // deposit_cap <= 0
}
```

También pueden aparecer errores de OpenZeppelin: `FungibleTokenError::InsufficientBalance` (100) al quemar más cuotas de las que tiene alguien, `PausableError::EnforcedPause` (1000) y los de Ownable (2100+).

## 8. Matemática de cuotas (`math.rs`)

Usar `stellar_contract_utils::math::i128_fixed_point::mul_div_with_rounding(e, x, y, denominador, Rounding::Floor | Rounding::Ceil)`. Es la misma fórmula de OpenZeppelin, pero con nuestro `total_assets`.

```
V = 10^DECIMALS_OFFSET
convert_to_shares(assets, redondeo) = mul_div(assets, total_supply + V, total_assets + 1, redondeo)
convert_to_assets(shares, redondeo) = mul_div(shares, total_assets + 1, total_supply + V, redondeo)

preview_deposit(assets)  = convert_to_shares(assets, Floor)   // cuotas que recibes
preview_withdraw(assets) = convert_to_shares(assets, Ceil)    // cuotas que se queman
preview_redeem(shares)   = convert_to_assets(shares, Floor)   // USDC que recibes
```

Regla: **el redondeo siempre favorece a la bóveda** (a los que se quedan).

## 9. Interfaz de estrategia (`strategy.rs`)

Igual a `DeFindexStrategyTrait` (repo `paltalabs/defindex`, `apps/contracts/strategies/core`). Declararla con `#[contractclient(name = "StrategyClient")]`:

```rust
pub trait Strategy {
    fn asset(e: Env) -> Address;
    fn deposit(e: Env, amount: i128, from: Address) -> i128;              // devuelve el saldo de `from` después
    fn withdraw(e: Env, amount: i128, from: Address, to: Address) -> i128;
    fn balance(e: Env, from: Address) -> i128;                            // valor en USDC de lo que tiene `from`
    fn harvest(e: Env, from: Address, data: Option<Bytes>);               // la bóveda NO la llama
}
```

### Comportamiento real de la estrategia Blend de DeFindex (verificado en su código)

1. **`deposit`** transfiere `amount` completo desde la bóveda y puede **devolver una pequeña parte** por redondeo. → La bóveda calcula lo invertido **midiendo su saldo de USDC antes y después**.
2. **`withdraw`** entrega a `to` un monto que puede ser **unas unidades menor** al pedido (redondeo hacia abajo). → La bóveda pide `WITHDRAW_ROUNDING_BUFFER` de más y **mide lo recibido**.
3. Montos muy chicos fallan (`InvalidSharesMinted` 416, `BTokensAmountBelowMin` 453). → La bóveda no mueve montos menores a `MIN_STRATEGY_MOVE`.
4. Si Blend no tiene liquidez, `withdraw` **falla**. → La bóveda usa `try_withdraw` y sigue con la siguiente estrategia.
5. `deposit`/`withdraw` exigen `from.require_auth()`. Como la bóveda es quien llama, esa firma es automática.

### Autorización al depositar en una estrategia (obligatorio)

Dentro de `strategy.deposit`, la estrategia ejecuta `usdc.transfer(bóveda → estrategia, amount)`. Esa transferencia **no** la invoca la bóveda directamente, así que la bóveda debe autorizarla justo antes, **solo por ese monto y hacia esa estrategia**:

```rust
e.authorize_as_current_contract(vec![
    &e,
    InvokerContractAuthEntry::Contract(SubContractInvocation {
        context: ContractContext {
            contract: usdc.clone(),
            fn_name: Symbol::new(&e, "transfer"),
            args: (e.current_contract_address(), strategy.clone(), amount).into_val(&e),
        },
        sub_invocations: vec![&e],
    }),
]);
StrategyClient::new(&e, &strategy).deposit(&amount, &e.current_contract_address());
```

`withdraw` no necesita esto: el USDC viene desde Blend hacia la bóveda.

## 10. Funciones del contrato

Notación: **Auth** = quién debe firmar · **Pausa** = si la bloquea `pause()`.

### 10.1 Constructor

```rust
fn __constructor(e: Env, asset: Address, owner: Address, strategy_timelock: u64, deposit_cap: i128)
```
- `strategy_timelock ≤ MAX_TIMELOCK_SECS`; si no → `InvalidTimelock`.
- `deposit_cap > 0`; si no → `InvalidDepositCap`. Testnet: `i128::MAX` (sin tope).
- Guarda `asset`, `timelock` y `deposit_cap`, fija el dueño (`ownable::set_owner`) y los metadatos del token (decimales = decimales del USDC + `DECIMALS_OFFSET`, nombre, símbolo).
- Testnet: `strategy_timelock = 60`. **Mainnet: mínimo 172 800 (48 h).** Es inmutable y visible en la blockchain.

### 10.2 Consultas (sin auth)

| Función | Devuelve |
|---|---|
| `query_asset() -> Address` | USDC |
| `total_assets() -> i128` | `idle_assets() + Σ StrategyClient.balance(bóveda)` (todas, incluso pausadas). Valoración **conservadora** (§10.9): una estrategia que no responde cuenta como 0 |
| `idle_assets() -> i128` | Saldo de USDC de la bóveda |
| `strategies() -> Vec<StrategyInfo>` | Lista de estrategias |
| `strategy_balances() -> Vec<(Address, i128)>` | Valor actual en cada estrategia (para el frontend) |
| `pending_strategy() -> Option<PendingStrategy>` | Estrategia propuesta |
| `strategy_timelock() -> u64` | Plazo de espera |
| `convert_to_shares(assets)`, `convert_to_assets(shares)` | Con redondeo hacia abajo |
| `preview_deposit`, `preview_withdraw`, `preview_redeem` | Según §8 |
| `max_withdraw(owner) -> i128` | `preview_redeem(balance(owner))`. No considera la liquidez de Blend: si falta, el retiro falla con `InsufficientLiquidity`. |
| `max_redeem(owner) -> i128` | `balance(owner)` |
| `deposit_cap() -> i128` | Tope actual |
| `max_deposit(receiver) -> i128` | `0` si está pausado o si alguna estrategia no responde (§10.9); si no, `max(deposit_cap − total_assets, 0)` (el mismo valor para todos) |
| `paused() -> bool`, `get_owner() -> Option<Address>` | De OpenZeppelin |

Además, el token de cuotas expone la interfaz SEP-41 completa vía `#[contractimpl(contracttrait)] impl FungibleToken` con `type ContractType = Base` (`balance`, `transfer`, `approve`, `decimals`, etc.).

### 10.3 `deposit(assets, receiver, from) -> i128`
- **Auth:** `from` · **Pausa:** sí
1. `assets > 0` (si no, `AmountMustBePositive`).
2. `total` = valoración **estricta** (§10.9): si alguna estrategia no responde → `StrategyUnavailable`. Si `total + assets > deposit_cap` → `DepositCapExceeded`. `preview_deposit` usa la misma valoración estricta.
3. `shares = preview_deposit(assets)`, **calculado antes de recibir el USDC**. Si `shares == 0` → `ZeroShares`.
4. `usdc.transfer(from → bóveda, assets)`.
5. `Base::mint(receiver, shares)`.
6. Evento `deposit`. Devuelve `shares`.
- **No** invierte en estrategias.

### 10.4 `withdraw(assets, receiver, owner) -> i128`
- **Auth:** `owner` · **Pausa:** no
1. `assets > 0`.
2. `shares = preview_withdraw(assets)`, calculado **antes** de mover nada.
3. `ensure_liquidity(assets)` (§10.6).
4. Quemar `shares` de `owner` (`Base::update(e, Some(&owner), None, shares)`). Si no tiene suficientes → error de OpenZeppelin 100.
5. `usdc.transfer(bóveda → receiver, assets)`.
6. Evento `withdraw`. Devuelve `shares`.

### 10.5 `redeem(shares, receiver, owner) -> i128`
- **Auth:** `owner` · **Pausa:** no
1. `shares > 0`.
2. `assets = preview_redeem(shares)`, calculado **antes** de mover nada. Si `assets == 0` → `ZeroAssets`.
3. `ensure_liquidity(assets)`.
4. Quemar `shares` de `owner`.
5. `usdc.transfer(bóveda → receiver, assets)`.
6. Evento `withdraw`. Devuelve `assets`.

### 10.6 `ensure_liquidity(needed)` (interna)
1. `idle = usdc.balance(bóveda)`. Si `idle ≥ needed`, termina.
2. Para cada estrategia **en orden de registro** (incluidas las pausadas):
   - `bal = balance(bóveda)` con `try_balance`; si falla o `bal == 0`, sigue con la siguiente.
   - `request = min(needed − idle + WITHDRAW_ROUNDING_BUFFER, bal)`.
   - `try_withdraw(request, bóveda, bóveda)`. Si falla, sigue con la siguiente.
   - `idle = usdc.balance(bóveda)` (**medir**, no confiar en el valor devuelto). Si `idle ≥ needed`, termina.
3. Si al final `idle < needed` → `InsufficientLiquidity`.

### 10.7 `rebalance()`
- **Auth:** ninguna (público) · **Pausa:** sí
1. `total = total_assets()` (conservadora), `tol = total × REBALANCE_TOLERANCE_BPS / BPS`. Las estrategias que no responden su saldo se **saltan**: no se les deposita ni se les retira (§10.9).
2. **Sacar de las que sobran:** para cada estrategia, `target = 0` si está pausada; si no, `total × target_bps / BPS`. Si `bal > target + tol` y `bal − target ≥ MIN_STRATEGY_MOVE`: `withdraw(bal − target)`.
3. **Invertir en las que faltan:** `idle = usdc.balance(bóveda)`, `idle_target = total × (BPS − Σ target_bps activos) / BPS`, `disponible = idle − idle_target`. Para cada estrategia **no pausada** con `bal < target − tol`: `monto = min(target − bal, disponible)`. Si `monto ≥ MIN_STRATEGY_MOVE`: autorizar (§9), `deposit(monto)`, y `disponible −= (idle_antes − idle_después)` (medido).
4. Evento `strategy_deposit` / `strategy_withdraw` por cada movimiento y un evento `rebalance` al final con `total_assets` antes y después.
- Si nada supera la tolerancia, no mueve dinero y no falla; igual emite `rebalance` (con `before == after`) para poder monitorearlo.
- Si una estrategia falla al depositar o retirar (`try_deposit` / `try_withdraw`), se salta y se sigue con las demás: una estrategia rota no bloquea el rebalance.

### 10.8 Administración (Auth: `owner`)

| Función | Qué hace | Validaciones |
|---|---|---|
| `propose_strategy(strategy, target_bps)` | Guarda `PendingStrategy { eta: ahora + timelock }` | sin otra pendiente (`PendingStrategyExists`); no registrada (`StrategyAlreadyExists`); menos de `MAX_STRATEGIES` (`TooManyStrategies`); `StrategyClient.asset() == asset` (`AssetMismatch`); nueva suma ≤ `BPS − MIN_IDLE_BPS` (`AllocationTooHigh`) |
| `cancel_strategy()` | Borra la pendiente | existe (`NoPendingStrategy`) |
| `accept_strategy()` | Agrega la pendiente a `Strategies` | existe; `ahora ≥ eta` (`TimelockNotExpired`); revalidar cupo y suma |
| `set_target(strategy, target_bps)` | Cambia el % objetivo | existe (`StrategyNotFound`); nueva suma ≤ `BPS − MIN_IDLE_BPS` |
| `pause_strategy(strategy)` / `unpause_strategy(strategy)` | Marca `paused` | existe |
| `remove_strategy(strategy)` | Saca todo de la estrategia y la quita de la lista | existe; después de sacar, `balance ≤ STRATEGY_DUST` (`StrategyNotEmpty`). El polvo restante se abandona (se documenta en el evento). |
| `set_deposit_cap(cap)` | Cambia el tope de depósitos | `cap > 0` (`InvalidDepositCap`). Puede quedar bajo el `total_assets` actual: eso solo impide nuevos depósitos, **nunca** afecta retiros. Sin plazo de espera. |
| `pause()` / `unpause()` | Pausa general (OpenZeppelin `pausable`) | — |
| `transfer_ownership(new_owner, live_until_ledger)` / `accept_ownership()` | Cambio de dueño en dos pasos (OpenZeppelin) | — |

No exponer `renounce_ownership`: sin dueño no se podría quitar una estrategia con problemas. Por eso las funciones de dueño se exponen a mano con las funciones libres de `stellar_access::ownable`, sin `impl Ownable` (que la exportaría).

### 10.9 Estrategia que no responde su saldo

Si `StrategyClient.balance(bóveda)` falla (p. ej. un problema en Blend), la bóveda no puede saber cuánto vale el fondo. Regla: **nadie entra a un precio incorrecto, pero todos pueden salir.**

| Operación | Valoración | Con una estrategia que no responde |
|---|---|---|
| `deposit`, `preview_deposit` | **Estricta** | Falla con `StrategyUnavailable` (6) |
| `max_deposit` | Estricta | Devuelve `0` |
| `withdraw`, `redeem` y sus `preview_*`, `max_withdraw` | **Conservadora** | La estrategia cuenta como 0: quien rescata recibe su parte de lo que **sí** está disponible |
| `total_assets`, `convert_to_*`, `strategy_balances` | Conservadora | Cuenta como 0 (en `strategy_balances` aparece con 0) |
| `ensure_liquidity` | — | Salta la estrategia |
| `rebalance` | Conservadora | No le deposita ni le retira |

- Implementación: `strategy_balance` usa `try_balance` y devuelve `Option<i128>`. Dos funciones internas: `total_assets_strict` (falla con `StrategyUnavailable`) y `total_assets_conservative` (cuenta 0).
- Consecuencia aceptada: quien rescata mientras la estrategia falla **renuncia a su parte de lo atascado**, y esa parte queda para los que se quedan. Quien prefiera esperar, no pierde nada. Así se cumple "rescate siempre disponible" sin que nadie pueda aprovecharse del precio.
- `remove_strategy` de una estrategia que no responde sigue fallando (no se puede comprobar que esté vacía). Ver §15.

## 11. Eventos (`events.rs`)

| Evento | Topics | Data |
|---|---|---|
| `deposit` | `from`, `receiver` | `assets`, `shares` |
| `withdraw` | `owner`, `receiver` | `assets`, `shares` |
| `strategy_deposit` | `strategy` | `amount` (medido) |
| `strategy_withdraw` | `strategy` | `amount` (medido) |
| `rebalance` | — | `total_assets_before`, `total_assets_after` |
| `strategy_proposed` | `strategy` | `target_bps`, `eta` |
| `strategy_cancelled` | `strategy` | — |
| `strategy_accepted` | `strategy` | `target_bps` |
| `strategy_removed` | `strategy` | `dust_left` |
| `target_changed` | `strategy` | `old_bps`, `new_bps` |
| `strategy_paused` / `strategy_unpaused` | `strategy` | — |
| `deposit_cap_changed` | — | `old_cap`, `new_cap` |

`Base::mint` emite `mint`; la quema con `Base::update` no emite evento propio (queda registrada en `withdraw`). Los eventos de `transfer`, pausa y dueño los emite OpenZeppelin.

## 12. Tests

### 12.1 Estrategia falsa para tests (`test/mock_strategy.rs`)

Contrato **solo para tests** (no se despliega; no es una "estrategia Fondex"). Implementa la interfaz de §9 y permite simular:
- **interés**: `simulate_yield(bps)` sube el valor de todos los saldos (acuña USDC de prueba a sí misma);
- **pérdida**: `simulate_loss(bps)`;
- **falta de liquidez**: `set_withdraw_cap(max)`; si se pide más, `withdraw` falla;
- **redondeo**: `set_deposit_refund(n)` devuelve `n` unidades al depositar; `set_withdraw_shortfall(n)` entrega `n` unidades menos al retirar.

### 12.2 Lista mínima de tests (cada uno con nombre propio)

**Cuotas y redondeo**
- primer depósito: 1 USDC → 10^17 cuotas;
- depositar y retirar de inmediato nunca devuelve más de lo depositado;
- `ZeroShares` y `ZeroAssets` con montos mínimos;
- después de `simulate_yield`, la cuota vale más y un nuevo depositante recibe menos cuotas.

**Estrategias**
- `rebalance` con 1000 USDC y objetivo 90% → ~900 en la estrategia y ~100 en reserva;
- `rebalance` con desviación menor a la tolerancia no mueve nada;
- `rebalance` con `deposit_refund` y `withdraw_shortfall` deja `total_assets` correcto (diferencia ≤ unas unidades a favor de la bóveda);
- estrategia pausada: `rebalance` la vacía y no le deposita;
- `withdraw` mayor que la reserva saca el faltante de la estrategia;
- `withdraw` con `withdraw_cap` y dos estrategias: usa la segunda si la primera falla;
- `withdraw` sin liquidez suficiente → `InsufficientLiquidity`;
- la autorización de `transfer` cubre exactamente `(bóveda, estrategia, monto)`: una estrategia que intente transferir más falla.

**Administración**
- cada función de admin falla sin la firma del dueño (árbol de auth exacto);
- `accept_strategy` antes del plazo → `TimelockNotExpired`; después funciona;
- `AssetMismatch`, `AllocationTooHigh`, `TooManyStrategies`, `StrategyAlreadyExists`;
- `remove_strategy` saca todo y quita la estrategia; con saldo mayor al polvo → `StrategyNotEmpty`.

**Tope de depósitos**
- depositar hasta exactamente el tope funciona; 1 unidad más → `DepositCapExceeded`;
- con interés acumulado, el tope cuenta el valor actual del fondo (`total_assets`), no lo depositado;
- bajar el tope bajo `total_assets` bloquea depósitos pero `withdraw` y `redeem` siguen funcionando;
- `max_deposit` coincide con lo que realmente se puede depositar;
- `set_deposit_cap` sin la firma del dueño falla.

**Estrategia que no responde (§10.9)** (perilla de la mock: `set_balance_fails(bool)`)
- `deposit` y `preview_deposit` fallan con `StrategyUnavailable`; `max_deposit` devuelve 0;
- `withdraw` y `redeem` funcionan con la reserva y otras estrategias, valorando la rota en 0;
- `rebalance` no falla y no toca la estrategia rota;
- cuando la estrategia vuelve a responder, los que se quedaron recuperan el valor completo.

**Pausa**
- pausado: `deposit` y `rebalance` fallan con 1000; `withdraw` y `redeem` **funcionan**.

**Invariantes** (repetidos con secuencias aleatorias de depósitos, retiros, rebalances, interés y pérdidas)
1. El USDC solo sale de la bóveda hacia el `receiver` de un retiro (quemando cuotas del `owner`) o hacia una estrategia registrada.
2. Ningún rol puede impedir un retiro si hay liquidez.
3. `rebalance` no baja `total_assets` más que el redondeo (≤ 2 × `WITHDRAW_ROUNDING_BUFFER`).
4. `Σ target_bps ≤ BPS − MIN_IDLE_BPS` siempre.
5. Nadie obtiene más USDC del que le corresponde por sus cuotas.

### 12.3 Prueba en testnet (`scripts/e2e-testnet.sh`)

Con `stellar` CLI contra la estrategia real de DeFindex:
1. build y deploy (`asset` = USDC de Blend, `owner` = cuenta admin, `timelock` = 60, `deposit_cap` = `i128::MAX`);
2. depósito inicial de Fondex (1 USDC) — mitiga el ataque de inflación;
3. `propose_strategy(CALLOM5…, 9000)` → esperar 60 s → `accept_strategy`;
4. un usuario deposita 100 USDC → `rebalance` → verificar ~90 en la estrategia;
5. esperar → `total_assets` sube (interés de Blend);
6. retirar 50 USDC (saca desde la estrategia) → retirar todo con `redeem`.

### 12.4 Redesplegar en testnet con un comando (`scripts/deploy-testnet.sh`)

Como el contrato no es actualizable, corregir algo en testnet = desplegar uno nuevo. El script deja todo listo sin pasos manuales:

1. `make ci` (si falla, no despliega);
2. `stellar contract build` y `stellar contract deploy` con los argumentos del constructor (`deposit_cap = i128::MAX`, `timelock = 60`);
3. `propose_strategy(estrategia DeFindex, 9000)` → esperar el timelock → `accept_strategy()`;
4. depósito inicial de Fondex (1 USDC);
5. imprime la nueva dirección y la agrega al registro del README (fecha, dirección);
6. opcionalmente actualiza `VITE_VAULT_CONTRACT_ID` en `app/.env.local` (con confirmación).

Configuración pública (sin claves) en `scripts/testnet.env`: identidad del admin en `stellar keys`, red `testnet`, dirección de la estrategia y del USDC. Las claves viven solo en `stellar keys`.

## 13. Estado de implementación

Todo lo descrito en este documento está implementado y probado:

- **84 tests** en `make ci`: cuotas y redondeo, estrategias, rebalance, administración, pausa, tope de depósitos, estrategia que no responde, e **invariantes** con 64 secuencias aleatorias de 40 operaciones (más una versión larga de 500 × 60 que se corre a mano).
- **Desplegado en testnet** (`CAEGDXJP6KXDLZIS4KOLMVM6IBGDZZDRFQTHCDXK6CZNNHJOVNT6K2ZS`) contra la estrategia real de DeFindex. La prueba de punta a punta pasó: aporte, rebalance 90/10, interés de Blend, rescate mayor que la reserva y rescate total.
- **App web** (`app/`) conectada: composición del fondo, aportar, rescatar y rebalancear.

## 14. Riesgos

| Riesgo | Mitigación |
|---|---|
| Blend sin liquidez (pool cerca de 95% prestado) | Reserva ≥ 5% (objetivo 10%); `InsufficientLiquidity` claro; la app muestra el máximo retirable |
| Deuda incobrable en Blend | El *backstop* de Blend absorbe primeras pérdidas; si no alcanza, baja el valor de la cuota para todos |
| Estrategia de DeFindex con un bug | Auditada (OtterSec, 2025-03), sin upgrade; Fondex puede pausarla y quitarla |
| Admin malicioso agrega una estrategia trampa | Timelock (48 h en mainnet) + multifirma + máximo 4 estrategias |
| Ataque de inflación del primer depósito | Offset de 10 decimales + depósito inicial de Fondex |
| Estrategia que no responde su saldo | Depósitos se bloquean; rescates siguen valorándola en 0 (§10.9) |
| Keeper de DeFindex deja de reinvertir BLND | Se pierde solo ese extra; el interés base sigue |
| Bug no detectado (el contrato no se puede parchear) | Tests + auditoría; en mainnet, `deposit_cap` bajo al inicio para limitar cuánto dinero está expuesto; si hay un bug: pausar, desplegar versión corregida y que los usuarios migren |

## 15. Preguntas abiertas (no bloquean la implementación)

1. Comisiones.
2. Legal: Ley 20.712 y Ley Fintec (21.521), antes de mainnet.
3. Quiénes firman la cuenta multifirma del admin en mainnet.
4. Estrategia de DeFindex para mainnet: buscar su dirección en `paltalabs/defindex/public/mainnet.contracts.json` y verificar el pool de Blend al que apunta.
5. ¿Qué hacer con una estrategia que deja de responder **para siempre**? Hoy no se puede quitar (§10.9). Opción futura: un `force_remove_strategy` con plazo de espera, que la saque de la lista sin retirar (lo que tenga queda a nombre de la bóveda y podría recuperarse si se vuelve a agregar).

## 16. Keeper: rebalanceo automático (pendiente)

Aportar no invierte (§10.3), y un contrato no puede ejecutarse solo: alguien tiene que llamar `rebalance()`. Hoy lo hace cualquiera desde la app (botón "Rebalancear"). En producción lo hará un **keeper**: un proceso de Fondex que revisa el fondo y llama `rebalance()` cuando hace falta, para que el usuario solo aporte y rescate.

- **Sin permisos especiales:** `rebalance()` es público. El keeper usa una cuenta propia con XLM solo para comisiones; **no** es la cuenta del dueño ni puede mover dinero fuera del fondo. Si el keeper se cae o lo comprometen, lo peor que pasa es que no se rebalancea (los rescates siguen funcionando), y cualquiera puede llamar `rebalance()` en su lugar.
- **Cuándo ejecutar:** cada N minutos (p. ej. 15), simula `strategies()`, `strategy_balances()`, `idle_assets()` y `total_assets()` y calcula la desviación como la app (`src/lib/composition.ts`). Solo envía la transacción si alguna fila supera `REBALANCE_TOLERANCE_BPS` y el movimiento sería ≥ `MIN_STRATEGY_MOVE`; así no paga comisiones de más. No ejecutar si `paused()`.
- **Dónde vive:** en el backend Rust (crate aparte, p. ej. `fondex-keeper`), con `stellar-rpc-client` / `soroban-client`, o un script programado al inicio. Configuración: RPC, contrato, identidad del keeper.
- **Monitoreo:** registrar cada evento `rebalance` (`total_assets_before/after`) y alertar si `total_assets_after < total_assets_before − 2 × WITHDRAW_ROUNDING_BUFFER`, si la cuenta del keeper se queda sin XLM o si `rebalance` falla varias veces seguidas.
- **App:** cuando el keeper esté activo, el botón "Rebalancear" se oculta o pasa a una sección avanzada.

## Fuentes (consultadas 2026-10-12)

- DeFindex: `paltalabs/defindex` (commit `a0fcabf`, 2026-07-01) — `strategies/core` (interfaz y errores), `strategies/blend/src/lib.rs` y `utils.rs` (comportamiento), `public/testnet.contracts.json` (direcciones). Licencia GPL-3.0: no copiamos código, solo llamamos al contrato desplegado.
- Blend v2: `blend-capital/blend-contracts-v2`, `blend-capital/blend-utils/testnet.contracts.json`, https://docs.blend.capital
- OpenZeppelin Stellar 0.7.2: `stellar-tokens` (fungible, vault), `stellar-access` (ownable), `stellar-contract-utils` (pausable, math).
- `soroban-sdk` 26.1.0: `Env::authorize_as_current_contract`.
- Consultas en vivo a testnet: estrategia `CALLOM5…` (asset = USDC de Blend, pool `TestnetV2`, sin función de upgrade) y pool `TestnetV2` (activo).
