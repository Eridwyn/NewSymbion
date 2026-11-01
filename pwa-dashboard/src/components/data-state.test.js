import { describe, it, expect } from 'vitest'
import { etatDepuisErreur } from './data-state.js'

describe('etatDepuisErreur()', () => {
  it('un fetch sans réponse = kernel injoignable', () => {
    expect(etatDepuisErreur(new TypeError('Failed to fetch')).etat).toBe('injoignable')
    expect(etatDepuisErreur(new Error('NetworkError when attempting to fetch resource.')).etat).toBe('injoignable')
    expect(etatDepuisErreur(new TypeError('Load failed')).etat).toBe('injoignable')
  })

  it('une réponse HTTP en erreur = erreur, avec le code, sans le texte anglais', () => {
    const { etat, message } = etatDepuisErreur(new Error('HTTP 500: Internal Server Error'))
    expect(etat).toBe('erreur')
    expect(message).toBe('Le kernel a répondu 500.')
  })

  it('les autres messages passent tels quels', () => {
    expect(etatDepuisErreur(new Error('Session expirée'))).toEqual({ etat: 'erreur', message: 'Session expirée' })
    expect(etatDepuisErreur(null).message).toBe('Erreur inconnue.')
  })
})

describe('<data-state>', () => {
  const monter = async (attrs) => {
    const el = document.createElement('data-state')
    Object.assign(el, attrs)
    document.body.appendChild(el)
    await el.updateComplete
    return el
  }

  it('propose « Réessayer » en cas d’échec, et le signale', async () => {
    const el = await monter({ etat: 'injoignable' })
    const bouton = el.shadowRoot.querySelector('button')
    expect(bouton.textContent).toBe('Réessayer')
    expect(el.shadowRoot.querySelector('[role="alert"]')).not.toBeNull()

    let recu = false
    el.addEventListener('reessayer', () => { recu = true })
    bouton.click()
    expect(recu).toBe(true)
  })

  it('ne propose rien à réessayer quand c’est seulement vide', async () => {
    const el = await monter({ etat: 'vide', titre: 'Aucune validation en attente' })
    expect(el.shadowRoot.querySelector('button')).toBeNull()
    expect(el.shadowRoot.querySelector('.titre').textContent).toBe('Aucune validation en attente')
  })
})
