import { describe, it, expect } from 'vitest'
import {
  escapeHtml,
  escapeHtmlPreserveNewlines,
  escapeAttribute,
} from './sanitization.js'

describe('escapeHtml', () => {
  it('escapes < and >', () => {
    expect(escapeHtml('<script>alert("xss")</script>')).not.toContain('<script>')
    expect(escapeHtml('<div>')).toBe('&lt;div&gt;')
  })

  it('escapes &', () => {
    expect(escapeHtml('a & b')).toBe('a &amp; b')
  })

  it('handles strings with quotes (passed through as-is in innerHTML)', () => {
    const result = escapeHtml('"hello"')
    expect(typeof result).toBe('string')
    expect(result).toContain('hello')
  })

  it('returns empty string for null', () => {
    expect(escapeHtml(null)).toBe('')
  })

  it('returns empty string for undefined', () => {
    expect(escapeHtml(undefined)).toBe('')
  })

  it('converts numbers to string', () => {
    expect(escapeHtml(42)).toBe('42')
  })

  it('preserves safe text', () => {
    expect(escapeHtml('Hello World')).toBe('Hello World')
  })
})

describe('escapeHtmlPreserveNewlines', () => {
  it('converts newlines to <br/>', () => {
    expect(escapeHtmlPreserveNewlines('line1\nline2')).toContain('<br/>')
  })

  it('escapes HTML and preserves newlines', () => {
    const result = escapeHtmlPreserveNewlines('<b>bold</b>\nnext')
    expect(result).not.toContain('<b>')
    expect(result).toContain('<br/>')
  })

  it('handles multiple newlines', () => {
    const result = escapeHtmlPreserveNewlines('a\nb\nc')
    expect(result.match(/<br\/>/g)).toHaveLength(2)
  })
})

describe('escapeAttribute', () => {
  it('escapes double quotes', () => {
    expect(escapeAttribute('say "hi"')).toBe('say &quot;hi&quot;')
  })

  it('escapes single quotes', () => {
    expect(escapeAttribute("it's")).toBe('it&#39;s')
  })

  it('escapes < and >', () => {
    expect(escapeAttribute('<tag>')).toBe('&lt;tag&gt;')
  })

  it('escapes &', () => {
    expect(escapeAttribute('a&b')).toBe('a&amp;b')
  })

  it('returns empty string for null', () => {
    expect(escapeAttribute(null)).toBe('')
  })

  it('returns empty string for undefined', () => {
    expect(escapeAttribute(undefined)).toBe('')
  })

  it('converts numbers to string and escapes', () => {
    expect(escapeAttribute(42)).toBe('42')
  })

  it('handles empty string', () => {
    expect(escapeAttribute('')).toBe('')
  })
})
