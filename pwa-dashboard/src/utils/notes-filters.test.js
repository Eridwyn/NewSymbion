import { describe, it, expect } from 'vitest'
import {
  filterByContext,
  filterBySearch,
  filterByTags,
  filterUrgent,
  extractAllTags,
  applyAllFilters,
} from './notes-filters.js'

// Helpers
const note = (overrides = {}) => ({
  data: { content: 'Test note', context: 'pro', tags: ['dev'], urgent: false, ...overrides },
})

describe('filterByContext', () => {
  const notes = [
    note({ context: 'pro' }),
    note({ context: 'maison' }),
    note({ context: 'veille' }),
    note({ context: 'focus' }),
  ]

  it('filters notes matching exact context', () => {
    expect(filterByContext(notes, 'focus')).toHaveLength(1)
  })

  it('supports MODE_ALIASES: pro matches cravate', () => {
    const mixed = [note({ context: 'cravate' }), note({ context: 'pro' })]
    expect(filterByContext(mixed, 'pro')).toHaveLength(2)
  })

  it('supports MODE_ALIASES: maison matches intime', () => {
    const mixed = [note({ context: 'intime' }), note({ context: 'maison' })]
    expect(filterByContext(mixed, 'maison')).toHaveLength(2)
  })

  it('supports MODE_ALIASES: veille matches neutre', () => {
    const mixed = [note({ context: 'neutre' }), note({ context: 'veille' })]
    expect(filterByContext(mixed, 'veille')).toHaveLength(2)
  })

  it('is case-insensitive', () => {
    expect(filterByContext(notes, 'PRO')).toHaveLength(1)
  })

  it('returns all notes for null/undefined context', () => {
    expect(filterByContext(notes, null)).toHaveLength(4)
    expect(filterByContext(notes, undefined)).toHaveLength(4)
  })

  it('returns empty array for non-array input', () => {
    expect(filterByContext(null, 'pro')).toEqual([])
    expect(filterByContext(undefined, 'pro')).toEqual([])
  })

  it('handles notes without context field', () => {
    const notesWithMissing = [note({ context: 'pro' }), note({ context: undefined })]
    expect(filterByContext(notesWithMissing, 'pro')).toHaveLength(1)
  })
})

describe('filterBySearch', () => {
  const notes = [
    note({ content: 'Réunion planning', context: 'pro', tags: ['meeting'] }),
    note({ content: 'Faire les courses', context: 'maison', tags: ['todo'] }),
    note({ content: 'Apprendre Rust', context: 'veille', tags: ['dev', 'learning'] }),
  ]

  it('searches in content', () => {
    expect(filterBySearch(notes, 'réunion')).toHaveLength(1)
  })

  it('searches in context', () => {
    expect(filterBySearch(notes, 'maison')).toHaveLength(1)
  })

  it('searches in tags', () => {
    expect(filterBySearch(notes, 'meeting')).toHaveLength(1)
  })

  it('is case-insensitive', () => {
    expect(filterBySearch(notes, 'RUST')).toHaveLength(1)
  })

  it('returns all notes for empty query', () => {
    expect(filterBySearch(notes, '')).toHaveLength(3)
    expect(filterBySearch(notes, '   ')).toHaveLength(3)
    expect(filterBySearch(notes, null)).toHaveLength(3)
  })

  it('returns empty for no matches', () => {
    expect(filterBySearch(notes, 'xyz123')).toHaveLength(0)
  })

  it('handles non-array input', () => {
    expect(filterBySearch(null, 'test')).toEqual([])
  })
})

describe('filterByTags', () => {
  const notes = [
    note({ tags: ['dev', 'rust'] }),
    note({ tags: ['todo'] }),
    note({ tags: ['dev', 'python'] }),
  ]

  it('filters notes with matching tags', () => {
    expect(filterByTags(notes, ['dev'])).toHaveLength(2)
    expect(filterByTags(notes, ['todo'])).toHaveLength(1)
  })

  it('matches any tag (OR logic)', () => {
    expect(filterByTags(notes, ['rust', 'todo'])).toHaveLength(2)
  })

  it('returns all notes for empty tag array', () => {
    expect(filterByTags(notes, [])).toHaveLength(3)
  })

  it('returns all notes for null/undefined tags', () => {
    expect(filterByTags(notes, null)).toHaveLength(3)
  })

  it('excludes notes without tags', () => {
    const withMissing = [...notes, note({ tags: undefined })]
    expect(filterByTags(withMissing, ['dev'])).toHaveLength(2)
  })
})

describe('filterUrgent', () => {
  it('returns only urgent notes', () => {
    const notes = [
      note({ urgent: true }),
      note({ urgent: false }),
      note({ urgent: true }),
    ]
    expect(filterUrgent(notes)).toHaveLength(2)
  })

  it('returns empty for no urgent notes', () => {
    expect(filterUrgent([note({ urgent: false })])).toHaveLength(0)
  })

  it('returns empty for non-array input', () => {
    expect(filterUrgent(null)).toEqual([])
    expect(filterUrgent(undefined)).toEqual([])
  })
})

describe('extractAllTags', () => {
  it('extracts unique tags sorted', () => {
    const notes = [
      note({ tags: ['b', 'a'] }),
      note({ tags: ['c', 'a'] }),
    ]
    expect(extractAllTags(notes)).toEqual(['a', 'b', 'c'])
  })

  it('handles notes without tags', () => {
    const notes = [note({ tags: ['x'] }), note({ tags: undefined })]
    expect(extractAllTags(notes)).toEqual(['x'])
  })

  it('returns empty for non-array input', () => {
    expect(extractAllTags(null)).toEqual([])
  })

  it('returns empty for empty notes', () => {
    expect(extractAllTags([])).toEqual([])
  })
})

describe('applyAllFilters', () => {
  const notes = [
    note({ content: 'Work task', context: 'pro', tags: ['dev'], urgent: true }),
    note({ content: 'Home task', context: 'maison', tags: ['todo'], urgent: false }),
    note({ content: 'Learn Rust', context: 'veille', tags: ['dev'], urgent: false }),
  ]

  it('returns all notes with no filters', () => {
    expect(applyAllFilters(notes, {})).toHaveLength(3)
  })

  it('applies context filter when enabled', () => {
    const result = applyAllFilters(notes, { contextFilterEnabled: true, context: 'pro' })
    expect(result).toHaveLength(1)
    expect(result[0].data.content).toBe('Work task')
  })

  it('ignores context filter when not enabled', () => {
    expect(applyAllFilters(notes, { context: 'pro' })).toHaveLength(3)
  })

  it('applies search filter', () => {
    expect(applyAllFilters(notes, { search: 'Rust' })).toHaveLength(1)
  })

  it('applies tag filter', () => {
    expect(applyAllFilters(notes, { tags: ['todo'] })).toHaveLength(1)
  })

  it('applies urgent filter', () => {
    expect(applyAllFilters(notes, { urgentOnly: true })).toHaveLength(1)
  })

  it('combines multiple filters', () => {
    const result = applyAllFilters(notes, {
      contextFilterEnabled: true,
      context: 'pro',
      urgentOnly: true,
    })
    expect(result).toHaveLength(1)
  })

  it('returns empty for non-array input', () => {
    expect(applyAllFilters(null)).toEqual([])
  })
})
