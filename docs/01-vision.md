# Fondex — Visión y decisiones iniciales

> Fondex = **Fon**dos mutuos **De**scentralizados sobre la red Stellar.
> Última actualización: 2026-10-12

## Visión

Que **cualquier persona pueda ahorrar e invertir** en un fondo mutuo: barato, transparente, sin montos mínimos y con rescate cuando quiera, usando contratos inteligentes.

Para Fondex, *descentralizado* significa **libertad financiera**: el usuario puede ver en todo momento dónde está su dinero y qué reglas lo gobiernan, porque esas reglas viven en contratos públicos.

## Perfil del producto: fondo mutuo

Fondex es un **fondo mutuo descentralizado**, no un fondo de inversión. En la práctica eso significa:

- **Liquidez primero:** el usuario puede rescatar en cualquier momento y se le paga de inmediato. El fondo mantiene una reserva para eso.
- **Bajo riesgo:** el objetivo es un rendimiento estable sobre dólares digitales, no maximizar ganancias.
- **Solo estrategias líquidas y de bajo riesgo**, según la política de inversión ([diseño técnico §2.1](02-diseno-tecnico.md)). Futuros, apalancamiento o activos volátiles quedan fuera de este fondo; si algún día se ofrecen, sería como un fondo distinto, con su propio perfil de riesgo.

## Decisiones tomadas

| Tema | Decisión |
|---|---|
| Red | Stellar, contratos inteligentes Soroban (Rust) |
| Tipo de producto | **Fondo mutuo descentralizado** (2026-10-12) |
| Backend | Rust |
| Frontend | React (apoyado con v0) |
| Cómo invierte el fondo | Bóveda propia + **estrategias de DeFindex** ya desplegadas (hoy: préstamos de USDC en Blend). Fondex no escribe estrategias propias por ahora. |
| Asignación | Fija, definida por el admin dentro de límites del contrato. Gestor / votaciones = feature futura. |
| Contrato actualizable | No |
| Marco legal | Chile — Ley 20.712 (fondos) y Ley Fintec (21.521). Por validar con asesoría legal. |
| Financiamiento | Interés en postular a financiamiento del ecosistema Stellar (SCF). |

## Pendientes (sin decidir)

- **Modelo económico:** qué comisiones se cobran, cuánto y cuándo.
- **Encaje legal exacto** del producto en la normativa chilena, incluido si se puede usar el término "fondo mutuo" en público.
- **Firmantes** de la cuenta multifirma del admin y **tope inicial** de depósitos para mainnet.

## Hoja de ruta (borrador)

1. **Fondo mutuo USDC** ✅ (testnet): aportar, ver tu participación y rescatar; inversión en Blend vía DeFindex, reserva de liquidez, rebalanceo y tope de depósitos ([diseño técnico](02-diseno-tecnico.md)). Todo verificable on-chain.
2. **Keeper:** bot de Fondex que ejecuta `rebalance` automáticamente (en el backend Rust); el usuario solo aporta y rescata ([diseño técnico §16](02-diseno-tecnico.md)).
3. **Comisiones y modelo económico.**
4. **Futuro:** más estrategias que cumplan la política, varios fondos, gobernanza / votaciones.
   - **Multiactivo:** fondos de otros activos que Blend presta (wBTC, wETH, XLM…), uno por activo. Partimos solo con USDC para simplificar el prototipo.
   - **Oráculos y RWA:** no están prohibidos, solo fuera del prototipo. Se necesitarán para fondos que mezclen activos y para RWA (activos del mundo real tokenizados).
5. **Largo plazo (si el producto evoluciona bastante):** servidor MCP en el backend Rust ([rmcp](https://github.com/modelcontextprotocol/rust-sdk)) para que los usuarios operen Fondex desde asistentes de IA (consultar cuotas, depositar con firma del usuario). No aplica a los contratos Soroban.

## Notas abiertas

- **Activos sintéticos:** un sintético sigue el precio de otro activo, así que mantiene el riesgo de mercado y agrega riesgo de oráculo, de colateral y de contraparte. Con el perfil de fondo mutuo, **no entran** en este fondo.
- **Oráculos:** no están prohibidos para Fondex; quedaron fuera del prototipo para no sumar riesgo. Cuando se usen (fondos que mezclen activos, RWA), elegir oráculos con varias fuentes y proteger contra precios manipulados o desactualizados.
- **Regulación:** administrar dinero de terceros en un fondo común cae bajo normativa de fondos (Ley 20.712), sea fondo mutuo o de inversión, además de la Ley Fintec. Requiere revisión con abogado especializado antes de operar con dinero real.
