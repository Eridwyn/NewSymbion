import { describe, it, expect } from 'vitest'
import {
  getDayNameShort,
  getDayNameFull,
  getAllDayNamesShort,
  getAllDayNamesFull,
  utcHourToLocal,
  localHourToUtc,
  formatHour,
  convertSignalsToLocal,
  convertPatternToLocal,
} from './time-utils.js'

describe('getDayNameShort', () => {
  it('returns correct short names for all days', () => {
    expect(getDayNameShort(0)).toBe('Lun')
    expect(getDayNameShort(1)).toBe('Mar')
    expect(getDayNameShort(2)).toBe('Mer')
    expect(getDayNameShort(3)).toBe('Jeu')
    expect(getDayNameShort(4)).toBe('Ven')
    expect(getDayNameShort(5)).toBe('Sam')
    expect(getDayNameShort(6)).toBe('Dim')
  })

  it('clamps out-of-range values', () => {
    expect(getDayNameShort(-1)).toBe('Lun')
    expect(getDayNameShort(7)).toBe('Dim')
    expect(getDayNameShort(100)).toBe('Dim')
  })

  it('defaults to Monday for null/undefined', () => {
    expect(getDayNameShort(null)).toBe('Lun')
    expect(getDayNameShort(undefined)).toBe('Lun')
  })
})

describe('getDayNameFull', () => {
  it('returns correct full names for all days', () => {
    expect(getDayNameFull(0)).toBe('Lundi')
    expect(getDayNameFull(1)).toBe('Mardi')
    expect(getDayNameFull(2)).toBe('Mercredi')
    expect(getDayNameFull(3)).toBe('Jeudi')
    expect(getDayNameFull(4)).toBe('Vendredi')
    expect(getDayNameFull(5)).toBe('Samedi')
    expect(getDayNameFull(6)).toBe('Dimanche')
  })

  it('clamps out-of-range values', () => {
    expect(getDayNameFull(-1)).toBe('Lundi')
    expect(getDayNameFull(7)).toBe('Dimanche')
  })

  it('defaults to Lundi for null/undefined', () => {
    expect(getDayNameFull(null)).toBe('Lundi')
    expect(getDayNameFull(undefined)).toBe('Lundi')
  })
})

describe('getAllDayNamesShort', () => {
  it('returns 7 short day names starting with Lun', () => {
    const names = getAllDayNamesShort()
    expect(names).toHaveLength(7)
    expect(names[0]).toBe('Lun')
    expect(names[6]).toBe('Dim')
  })

  it('returns a copy (not the original array)', () => {
    const a = getAllDayNamesShort()
    const b = getAllDayNamesShort()
    expect(a).not.toBe(b)
    expect(a).toEqual(b)
  })
})

describe('getAllDayNamesFull', () => {
  it('returns 7 full day names starting with Lundi', () => {
    const names = getAllDayNamesFull()
    expect(names).toHaveLength(7)
    expect(names[0]).toBe('Lundi')
    expect(names[6]).toBe('Dimanche')
  })
})

describe('utcHourToLocal / localHourToUtc', () => {
  it('roundtrips correctly', () => {
    for (let h = 0; h < 24; h++) {
      expect(utcHourToLocal(localHourToUtc(h))).toBe(h)
    }
  })

  it('returns values in 0-23 range', () => {
    for (let h = 0; h < 24; h++) {
      const local = utcHourToLocal(h)
      expect(local).toBeGreaterThanOrEqual(0)
      expect(local).toBeLessThanOrEqual(23)
    }
  })
})

describe('formatHour', () => {
  it('formats hour without minutes by default', () => {
    expect(formatHour(14)).toBe('14h')
    expect(formatHour(0)).toBe('0h')
    expect(formatHour(23)).toBe('23h')
  })

  it('formats hour with minutes when requested', () => {
    expect(formatHour(14, true)).toBe('14:00')
    expect(formatHour(0, true)).toBe('0:00')
  })

  it('clamps out-of-range values', () => {
    expect(formatHour(-5)).toBe('0h')
    expect(formatHour(30)).toBe('23h')
  })

  it('handles null/undefined', () => {
    expect(formatHour(null)).toBe('0h')
    expect(formatHour(undefined)).toBe('0h')
  })
})

describe('convertSignalsToLocal', () => {
  it('returns all expected fields', () => {
    const result = convertSignalsToLocal({ hour: 12, day_of_week: 2 })
    expect(result).toHaveProperty('localHour')
    expect(result).toHaveProperty('dayNameShort', 'Mer')
    expect(result).toHaveProperty('dayNameFull', 'Mercredi')
    expect(result).toHaveProperty('displayText')
    expect(result.displayText).toContain('Mercredi')
    expect(result.displayText).toContain('h')
  })

  it('handles missing fields with defaults', () => {
    const result = convertSignalsToLocal({})
    expect(result.dayNameShort).toBe('Lun')
    expect(result.dayNameFull).toBe('Lundi')
  })
})

describe('convertPatternToLocal', () => {
  it('returns localHour, dayNameShort, dayNameFull', () => {
    const result = convertPatternToLocal({ hour: 8, day_of_week: 4 })
    expect(result).toHaveProperty('localHour')
    expect(result.dayNameShort).toBe('Ven')
    expect(result.dayNameFull).toBe('Vendredi')
  })

  it('handles missing fields', () => {
    const result = convertPatternToLocal({})
    expect(result.dayNameShort).toBe('Lun')
  })
})
