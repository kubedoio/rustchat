import { describe, it, expect } from 'vitest'
import { serializeQueryParams, buildURL } from '../querySerializer'

describe('querySerializer', () => {
  describe('serializeQueryParams', () => {
    it('serializes simple key-value pairs', () => {
      const result = serializeQueryParams({ page: 1, limit: 10 })
      expect(result).toBe('page=1&limit=10')
    })

    it('converts arrays to repeated keys', () => {
      const result = serializeQueryParams({ tags: ['a', 'b', 'c'] })
      expect(result).toBe('tags=a&tags=b&tags=c')
    })

    it('omits null and undefined values', () => {
      const result = serializeQueryParams({
        q: 'search',
        empty: null,
        missing: undefined,
        present: 'value',
      })
      expect(result).toBe('q=search&present=value')
      expect(result).not.toContain('empty')
      expect(result).not.toContain('missing')
    })

    it('converts dates to ISO strings', () => {
      const date = new Date('2024-01-15T10:30:00.000Z')
      const result = serializeQueryParams({ since: date })
      expect(result).toBe('since=2024-01-15T10%3A30%3A00.000Z')
    })

    it('converts booleans to string representations', () => {
      const result = serializeQueryParams({ active: true, deleted: false })
      expect(result).toBe('active=true&deleted=false')
    })

    it('JSON stringifies objects', () => {
      const result = serializeQueryParams({ filter: { status: 'active', role: 'admin' } })
      expect(result).toBe('filter=%7B%22status%22%3A%22active%22%2C%22role%22%3A%22admin%22%7D')
    })

    it('URL encodes special characters', () => {
      const result = serializeQueryParams({ q: 'hello world & more!' })
      expect(result).toBe('q=hello%20world%20%26%20more!')
    })

    it('returns empty string for empty params', () => {
      const result = serializeQueryParams({})
      expect(result).toBe('')
    })

    it('handles arrays with null/undefined items', () => {
      const result = serializeQueryParams({ tags: ['a', null, 'b', undefined, 'c'] })
      expect(result).toBe('tags=a&tags=b&tags=c')
    })

    it('handles nested arrays', () => {
      const result = serializeQueryParams({
        matrix: [
          ['a', 'b'],
          ['c', 'd'],
        ],
      })
      // Nested arrays become stringified
      expect(result).toContain('matrix=')
    })

    it('handles empty strings', () => {
      const result = serializeQueryParams({ q: '', empty: '' })
      expect(result).toBe('q=&empty=')
    })

    it('handles numbers including zero', () => {
      const result = serializeQueryParams({ page: 0, count: 10, negative: -5 })
      expect(result).toBe('page=0&count=10&negative=-5')
    })
  })

  describe('buildURL', () => {
    it('joins baseURL and path correctly', () => {
      const url = buildURL('https://api.example.com', '/users', undefined)
      expect(url).toBe('https://api.example.com/users')
    })

    it('handles baseURL with trailing slash', () => {
      const url = buildURL('https://api.example.com/', '/users', undefined)
      expect(url).toBe('https://api.example.com/users')
    })

    it('handles path without leading slash', () => {
      const url = buildURL('https://api.example.com', 'users', undefined)
      expect(url).toBe('https://api.example.com/users')
    })

    it('appends query string with ? separator', () => {
      const url = buildURL('https://api.example.com', '/users', { page: 1 })
      expect(url).toBe('https://api.example.com/users?page=1')
    })

    it('handles URL without baseURL', () => {
      const url = buildURL(undefined, 'https://api.example.com/users', { page: 1 })
      expect(url).toBe('https://api.example.com/users?page=1')
    })

    it('uses & separator for existing query params', () => {
      const url = buildURL('https://api.example.com', '/users?sort=name', { page: 1 })
      expect(url).toBe('https://api.example.com/users?sort=name&page=1')
    })
  })
})
