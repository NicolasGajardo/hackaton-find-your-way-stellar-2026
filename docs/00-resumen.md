# Fondex — Resumen: idea y modelo de negocio

> Más detalle en [01-vision.md](01-vision.md) y en el [diseño técnico](02-diseno-tecnico.md).

## 1. La idea en una frase

**Fondex es un fondo mutuo descentralizado: cualquier persona deposita dólares digitales, gana un rendimiento de bajo riesgo y puede rescatar cuando quiera, con reglas que impone un contrato público en Stellar y no un intermediario en quien haya que confiar.**

## 2. La idea desglosada

### El problema

Ahorrar e invertir en fondos hoy es **caro y poco accesible**: montos mínimos, comisiones poco claras, y el inversionista no ve dónde está su dinero ni puede verificar las reglas que lo gobiernan.

### La solución: un fondo mutuo en un contrato inteligente

1. El usuario **deposita USDC** (dólares digitales), sin monto mínimo.
2. Recibe **cuotas** (`fxFUND`) que representan su parte del fondo.
3. El fondo invierte en **estrategias líquidas y de bajo riesgo** (hoy: préstamos de USDC en Blend, a través de una estrategia auditada de DeFindex) y guarda una **reserva** para pagar rescates.
4. Los intereses suben el **valor de la cuota**.
5. El usuario **rescata cuando quiere**: el contrato le paga de inmediato.

### ¿Por qué "fondo mutuo" y no "fondo de inversión"?

Por cómo funciona, Fondex calza con un **fondo mutuo**: rescate en cualquier momento, valor cuota calculado continuamente, inversiones líquidas y de bajo riesgo, y una reserva para pagar rescates. En particular, se parece a un **fondo mutuo de dólares de corto plazo** (tipo *money market*). Un fondo de inversión, en cambio, suele aceptar activos menos líquidos, plazos de rescate más largos y más riesgo.

### Regla de oro: solo estrategias líquidas y de bajo riesgo

El fondo solo invierte en estrategias que cumplen **todos** los criterios de la **política de inversión** ([diseño técnico §2.1](02-diseno-tecnico.md)): mismo activo que el fondo (USDC), retiro inmediato, sin apalancamiento ni derivados, sin exposición al precio de otros activos, y contratos auditados con uso real. Futuros, apalancamiento o activos volátiles **no** entran en este fondo; si algún día se ofrecen, sería en **otro fondo** con otro perfil de riesgo.

### Hacia dónde crece (features futuras)

Partimos solo con USDC para **simplificar y llegar rápido a un prototipo funcional**, no porque Fondex vaya a ser solo de USDC.

- **Multiactivo:** Blend también presta otros activos (wBTC, wETH, XLM, etc.). La idea es soportarlos más adelante. Como el contrato no es actualizable, la forma natural es **un fondo por activo**: la misma estructura de contrato desplegada con otro activo y otra estrategia (p. ej. un "fondo wBTC" que presta wBTC en Blend). Requiere una versión nueva del contrato con algunos montos mínimos configurables, porque hoy están pensados para USDC. Un fondo que **mezcle** activos (p. ej. 50% USDC + 50% wBTC) es más complejo: necesita precios para valorar la cuota y sería un contrato nuevo.
- **Oráculos y RWA:** los oráculos (fuentes de precios) **no están prohibidos**; solo quedaron fuera del prototipo para no sumar riesgo y complejidad. Serán necesarios para fondos que mezclen activos y para **RWA** (activos del mundo real tokenizados, como bonos o letras del tesoro en la blockchain), que son un buen calce con el perfil de fondo mutuo.
- **Perfil de riesgo:** un fondo de wBTC o wETH sube y baja con el precio de ese activo, así que **no** sería un fondo de bajo riesgo como el de USDC. Se ofrecería como un fondo distinto, con su perfil de riesgo explicado al usuario.

### Principio central: "Fondex opera, el usuario es dueño"

Fondex puede **invertir** el dinero, pero nunca **llevárselo**. Lo garantiza el contrato, no una promesa:

- **El dinero solo va a estrategias aprobadas** o de vuelta a quien rescata sus cuotas.
- **Agregar una estrategia tiene plazo de espera público**, para que cualquiera alcance a rescatar si no está de acuerdo.
- **Rescate siempre disponible:** ningún rol puede bloquearlo (la pausa solo detiene depósitos).
- **Contrato no actualizable:** nadie puede cambiar las reglas después de desplegarlo.
- **Comisiones explícitas:** si existen, están escritas en el contrato.

Para Fondex, *descentralizado* significa **libertad financiera**: el usuario puede ver en todo momento dónde está su dinero y qué reglas lo gobiernan.

### Hoja de ruta

| Qué | Estado |
|---|---|
| Fondo mutuo USDC: aportes, cuotas, rescates, inversión en Blend vía DeFindex, reserva, rebalanceo, admin con plazo de espera, pausa y tope de depósitos | ✅ Contrato y app en testnet |
| **Keeper**: bot de Fondex que ejecuta `rebalance` automáticamente, para que el usuario solo aporte y rescate | Próximo ([diseño §16](02-diseno-tecnico.md)) |
| Comisiones y modelo económico | Próximo |
| Más estrategias que cumplan la política, **fondos de otros activos** (wBTC, wETH, XLM…), **RWA** (activos del mundo real tokenizados) con oráculos, gobernanza | Futuro (ver "Hacia dónde crece") |
| Operar Fondex desde asistentes de IA (MCP) | Largo plazo |

### Tecnología

Stellar (comisiones de fracciones de centavo, USDC nativo). Contratos Soroban (Rust), backend Rust, frontend React. Se apoya en protocolos existentes del ecosistema: **Blend** (préstamos) y **estrategias de DeFindex** (llamadas directamente, sin su API).

## 3. Modelo de negocio

### Decidido

- Fondex **gana con comisiones y/o acuerdos de uso**.
- Toda comisión es **una regla pública en el contrato**, con un tope máximo; nunca un cobro discrecional.

### Abierto: qué cobrar

| Comisión | Cómo funciona | A favor | En contra |
|---|---|---|---|
| **Administración** | % anual sobre lo administrado, cobrado de a poco | Ingreso predecible; estándar de los fondos mutuos | Se cobra aunque el fondo pierda |
| **Desempeño** | % de las ganancias | Alinea a Fondex con el usuario | Ingreso irregular; más complejo de calcular on-chain |
| **Depósito** | % al entrar | Simple | Desincentiva entrar: contrario a "que cualquiera invierta" |
| **Rescate** | % al salir | Simple | Choca con "rescata cuando quieras" |

**Propuesta (por discutir):** una **comisión de administración baja** (ej. < 1% anual) como base. Es coherente con la promesa de ser barato y fácil de explicar. Técnicamente se cobra emitiendo cuotas nuevas a Fondex, así el dinero nunca sale de la bóveda y se mantiene el principio "no puede llevárselo". Ojo: en un fondo de bajo riesgo el rendimiento es acotado, así que la comisión debe ser baja para que el usuario igual gane.

### Otras fuentes de ingreso o financiamiento

- **"Acuerdos de uso":** sin definir. Podría ser cobrar a terceros por usar la infraestructura de Fondex.
- **Financiamiento del ecosistema Stellar (SCF):** interés en postular. No es ingreso recurrente, pero puede financiar el desarrollo inicial.

## 4. Riesgos y preguntas abiertas

- **Legal:** en Chile, la Ley 20.712 regula **tanto fondos mutuos como fondos de inversión** (administrados por una AGF fiscalizada por la CMF), además de la Ley Fintec (21.521). Cambiar el enfoque a "fondo mutuo" es más preciso conceptualmente, pero **no saca a Fondex de la regulación**, y "fondo mutuo" es un término legal. Validar con abogado antes de operar con dinero real o de usar el término en público. **Probablemente el riesgo más grande del modelo de negocio.**
- **Riesgo de los protocolos usados:** deuda incobrable o falta de liquidez en Blend; un bug en la estrategia de DeFindex. Mitigado con la reserva, la política de inversión, el tope de depósitos y la posibilidad de pausar o quitar estrategias.
- **"Bajo riesgo" no es "sin riesgo":** el valor de la cuota puede bajar si un protocolo sufre pérdidas. Hay que comunicarlo claramente al usuario.
- **Por decidir:** comisiones, firmantes de la cuenta multifirma del admin, y tope inicial de depósitos para mainnet.
