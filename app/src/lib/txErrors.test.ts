import { describe, expect, it } from 'vitest'
import { describeTxError } from './txErrors'

describe('describeTxError', () => {
  it('traduce errores del token USDC', () => {
    const err = new Error('HostError: Error(Contract, #10)\nEvent log...')
    expect(describeTxError(err)).toBe('No tienes suficiente USDC para este aporte.')
  })

  it('traduce la falta de liquidez de la bóveda', () => {
    expect(describeTxError(new Error('Error(Contract, #4)'))).toBe(
      'Ahora mismo no hay liquidez suficiente en Blend para pagar este rescate. Prueba con un monto menor o inténtalo más tarde.',
    )
  })

  it('traduce la pausa del fondo', () => {
    expect(describeTxError(new Error('Error(Contract, #1000)'))).toBe(
      'El fondo está pausado: los aportes y el rebalanceo están detenidos. Los rescates siguen disponibles.',
    )
  })

  it('deja en el mensaje genérico los códigos que no usa la bóveda', () => {
    expect(describeTxError(new Error('Error(Contract, #405)'))).toBe(
      'El contrato rechazó la operación (código 405).',
    )
  })

  it('muestra el código si el error de contrato no es conocido', () => {
    expect(describeTxError(new Error('Error(Contract, #14)'))).toBe(
      'El contrato rechazó la operación (código 14).',
    )
  })

  it('detecta que el usuario canceló la firma', () => {
    expect(describeTxError(new Error('User declined access'))).toBe(
      'Cancelaste la firma en tu wallet.',
    )
  })

  it('lee errores del kit que vienen como objeto { code, message }', () => {
    const err = { code: -3, message: 'The site is not connected to Freighter' }
    expect(describeTxError(err)).toBe(
      'Tu wallet no ha autorizado este sitio. Desconecta y vuelve a conectar la wallet.',
    )
  })
})
