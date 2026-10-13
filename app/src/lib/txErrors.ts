/** Códigos de error de la bóveda y del token USDC. */
const CONTRACT_ERRORS: Record<number, string> = {
  1: 'El monto debe ser mayor que cero.',
  2: 'El aporte es demasiado pequeño para recibir cuotas.',
  3: 'El rescate es demasiado pequeño.',
  4: 'Ahora mismo no hay liquidez suficiente en Blend para pagar este rescate. Prueba con un monto menor o inténtalo más tarde.',
  5: 'El aporte supera el tope actual del fondo.',
  6: 'Una inversión del fondo no está respondiendo; los aportes están detenidos por seguridad. Los rescates siguen disponibles.',
  10: 'No tienes suficiente USDC para este aporte.',
  13: 'Tu cuenta no tiene la trustline de USDC. Agrégala en tu wallet.',
  100: 'No tienes suficientes cuotas para esta operación.',
  1000: 'El fondo está pausado: los aportes y el rebalanceo están detenidos. Los rescates siguen disponibles.',
  1500: 'El monto es demasiado grande.',
}

/** La cuenta activa en la wallet ya no es la que muestra la app. */
export const WALLET_ACCOUNT_CHANGED = 'WALLET_ACCOUNT_CHANGED'

/**
 * Traduce un error de firma / simulación / envío a un mensaje legible.
 * Soroban reporta los errores de contrato como `Error(Contract, #N)`.
 * Los códigos 10 a 20 de administración de la bóveda no se usan desde la app
 * y caen en el mensaje genérico, salvo 10 y 13, que son del token USDC.
 */
export function describeTxError(err: unknown): string {
  // El kit rechaza con objetos `{ code, message }` que no son instancias de Error.
  const message =
    err instanceof Error
      ? err.message
      : typeof err === 'object' && err !== null && 'message' in err
        ? String(err.message)
        : String(err)

  if (message === WALLET_ACCOUNT_CHANGED) {
    return 'Cambiaste de cuenta en tu wallet. Desconecta y vuelve a conectar.'
  }
  if (/not (connected|allowed)|request access/i.test(message)) {
    return 'Tu wallet no ha autorizado este sitio. Desconecta y vuelve a conectar la wallet.'
  }

  const contractCode = /Error\(Contract, #(\d+)\)/.exec(message)
  if (contractCode) {
    const code = Number(contractCode[1])
    return CONTRACT_ERRORS[code] ?? `El contrato rechazó la operación (código ${code}).`
  }

  if (/reject|declin|denied|cancel/i.test(message)) {
    return 'Cancelaste la firma en tu wallet.'
  }
  if (/trustline/i.test(message)) {
    return CONTRACT_ERRORS[13]
  }
  if (/Account not found|Resource missing/i.test(message)) {
    return 'Tu cuenta no existe en testnet. Fondéala con Friendbot primero.'
  }

  return message || 'La transacción falló.'
}
