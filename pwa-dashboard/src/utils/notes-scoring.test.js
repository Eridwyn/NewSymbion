import { describe, it, expect, vi } from 'vitest'
import {
  calculatePriorityScore,
  sortNotesByPriority,
  isHighPriority,
  getTopPriorityNotes,
} from './notes-scoring.js'

// Helper: create a note with a recent timestamp (today)
function recentNote(overrides = {}) {
  const now = new Date()
  const dayOfYear = Math.floor((now - new Date(now.getFullYear(), 0, 0)) / (1000 * 60 * 60 * 24))
  return {
    data: { content: 'Test', context: 'pro', urgent: false, ...overrides },
    timestamp: [now.getFullYear(), dayOfYear, now.getHours(), now.getMinutes(), 0, 0, 0, 0, 0],
  }
}

// Helper: note with old timestamp (30 days ago)
function oldNote(overrides = {}) {
  const d = new Date()
  d.setDate(d.getDate() - 30)
  const dayOfYear = Math.floor((d - new Date(d.getFullYear(), 0, 0)) / (1000 * 60 * 60 * 24))
  return {
    data: { content: 'Old', context: 'veille', urgent: false, ...overrides },
    timestamp: [d.getFullYear(), dayOfYear, d.getHours(), d.getMinutes(), 0, 0, 0, 0, 0],
  }
}

describe('calculatePriorityScore', () => {
  it('adds 100 points for urgent notes', () => {
    const score = calculatePriorityScore(recentNote({ urgent: true }), 'veille')
    expect(score).toBeGreaterThanOrEqual(100)
  })

  it('adds 50 points for context match', () => {
    const matching = calculatePriorityScore(recentNote({ context: 'pro' }), 'pro')
    const notMatching = calculatePriorityScore(recentNote({ context: 'pro' }), 'maison')
    expect(matching - notMatching).toBe(50)
  })

  it('gives recency score for recent notes', () => {
    const recent = calculatePriorityScore(recentNote(), 'other')
    const old = calculatePriorityScore(oldNote(), 'other')
    expect(recent).toBeGreaterThan(old)
  })

  it('gives 0 recency for old notes (>7 days)', () => {
    const score = calculatePriorityScore(oldNote(), 'other')
    expect(score).toBe(0)
  })

  it('handles note without timestamp', () => {
    const n = { data: { urgent: false, context: 'pro' } }
    expect(calculatePriorityScore(n, 'pro')).toBe(50)
  })

  it('handles note with null timestamp', () => {
    const n = { data: { urgent: false, context: 'pro' }, timestamp: null }
    expect(calculatePriorityScore(n, 'pro')).toBe(50)
  })

  it('returns 0 for note with no matching attributes and old timestamp', () => {
    expect(calculatePriorityScore(oldNote(), 'other')).toBe(0)
  })
})

describe('sortNotesByPriority', () => {
  it('sorts urgent notes first', () => {
    const notes = [
      recentNote({ urgent: false, context: 'veille' }),
      recentNote({ urgent: true, context: 'veille' }),
    ]
    const sorted = sortNotesByPriority(notes, 'veille')
    expect(sorted[0].data.urgent).toBe(true)
  })

  it('sorts context-matching notes before non-matching', () => {
    const notes = [
      oldNote({ context: 'maison' }),
      oldNote({ context: 'pro' }),
    ]
    const sorted = sortNotesByPriority(notes, 'pro')
    expect(sorted[0].data.context).toBe('pro')
  })

  it('does not mutate original array', () => {
    const notes = [recentNote(), oldNote()]
    const original = [...notes]
    sortNotesByPriority(notes, 'pro')
    expect(notes).toEqual(original)
  })

  it('returns empty for non-array input', () => {
    expect(sortNotesByPriority(null, 'pro')).toEqual([])
    expect(sortNotesByPriority(undefined, 'pro')).toEqual([])
  })
})

describe('isHighPriority', () => {
  it('returns true for context-matching non-urgent note', () => {
    expect(isHighPriority(recentNote({ context: 'pro' }), 'pro')).toBe(true)
  })

  it('returns false for urgent notes (even with high score)', () => {
    expect(isHighPriority(recentNote({ urgent: true, context: 'pro' }), 'pro')).toBe(false)
  })

  it('returns false for low-score notes', () => {
    expect(isHighPriority(oldNote({ context: 'veille' }), 'pro')).toBe(false)
  })
})

describe('getTopPriorityNotes', () => {
  it('returns top N notes by priority', () => {
    const notes = [
      oldNote({ context: 'veille' }),
      recentNote({ urgent: true, context: 'pro' }),
      recentNote({ context: 'pro' }),
      oldNote({ context: 'maison' }),
    ]
    const top = getTopPriorityNotes(notes, 'pro', 2)
    expect(top).toHaveLength(2)
    expect(top[0].data.urgent).toBe(true)
  })

  it('defaults to limit=3', () => {
    const notes = Array.from({ length: 10 }, () => recentNote())
    expect(getTopPriorityNotes(notes, 'pro')).toHaveLength(3)
  })

  it('returns all if fewer than limit', () => {
    const notes = [recentNote()]
    expect(getTopPriorityNotes(notes, 'pro', 5)).toHaveLength(1)
  })
})
