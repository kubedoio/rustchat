// @vitest-environment jsdom

import { mount, flushPromises } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { serializeQueryParams } from '../../api/http/querySerializer'

const apiGet = vi.fn()

vi.mock('../../api/client', () => ({
  default: {
    get: (...args: unknown[]) => apiGet(...args),
  },
}))

vi.mock('../../composables/useToast', () => ({
  useToast: () => ({
    error: vi.fn(),
    success: vi.fn(),
  }),
}))

// jsdom does not implement URL.createObjectURL; capture the blobs instead.
const createdUrls: Blob[] = []
const originalCreateObjectURL = URL.createObjectURL
const originalRevokeObjectURL = URL.revokeObjectURL

beforeEach(() => {
  apiGet.mockReset()
  createdUrls.length = 0
  URL.createObjectURL = vi.fn((blob: Blob) => {
    createdUrls.push(blob)
    return 'blob:mock'
  }) as unknown as typeof URL.createObjectURL
  URL.revokeObjectURL = vi.fn()
})

afterEach(() => {
  URL.createObjectURL = originalCreateObjectURL
  URL.revokeObjectURL = originalRevokeObjectURL
})

const auditLogs = [
  {
    id: 'log-1',
    action: 'add_member',
    status: 'success',
    target_type: 'channel',
    created_at: '2026-09-28T00:00:00Z',
  },
  {
    id: 'log-2',
    action: 'remove_member',
    status: 'failed',
    target_type: 'team',
    created_at: '2026-09-27T00:00:00Z',
  },
]

function stubApi() {
  // Emulate HttpClient semantics faithfully: with responseType 'blob' the
  // client resolves response.data to a Blob (HttpClient.ts: `await
  // response.blob()`). This is what made the old export code produce "{}"
  // (JSON.stringify of a Blob) — the mock must reproduce that behavior for
  // the regression assertion to be meaningful.
  apiGet.mockImplementation((url: string, config?: { responseType?: string }) => {
    if (url === '/admin/audit/membership/export') {
      if (config?.responseType === 'blob') {
        return Promise.resolve({
          data: new Blob([JSON.stringify(auditLogs)], { type: 'application/json' }),
        })
      }
      return Promise.resolve({ data: auditLogs })
    }
    if (url === '/admin/audit/membership/summary') {
      return Promise.resolve({
        data: {
          total_operations_24h: 2,
          successful_operations_24h: 1,
          failed_operations_24h: 1,
          failure_rate_24h: 50,
          pending_operations: 0,
          policies_with_failures: 1,
        },
      })
    }
    return Promise.resolve({ data: [] })
  })
}

async function mountView() {
  const AuditDashboard = (await import('./AuditDashboard.vue')).default
  const wrapper = mount(AuditDashboard, {
    attachTo: document.body,
  })
  await flushPromises()
  return wrapper
}

describe('AuditDashboard export', () => {
  it('exports the actual audit log entries as valid JSON, not "{}"', async () => {
    stubApi()
    const wrapper = await mountView()

    const exportButton = wrapper.findAll('button').find(b => b.text().includes('Export'))
    expect(exportButton).toBeDefined()
    await exportButton!.trigger('click')
    await flushPromises()

    // Guard 1 (call-site contract): the export request must not ask for a
    // blob — the endpoint returns JSON.
    const exportCall = apiGet.mock.calls.find(c => c[0] === '/admin/audit/membership/export')
    expect(exportCall).toBeDefined()
    const config = exportCall![1] as Record<string, unknown> | undefined
    expect(config?.responseType).not.toBe('blob')

    // Guard 2 (content, regression-meaningful because the mock emulates
    // HttpClient's blob behavior): with the old code (responseType 'blob'
    // + JSON.stringify) the downloaded file was "{}".
    expect(createdUrls).toHaveLength(1)
    const exported = await createdUrls[0].text()
    const parsed = JSON.parse(exported)
    expect(parsed).toEqual(auditLogs)

    wrapper.unmount()
  })

  it('sends export and list requests that satisfy the backend AuditLogQuery contract', async () => {
    stubApi()
    const wrapper = await mountView()

    const exportButton = wrapper.findAll('button').find(b => b.text().includes('Export'))
    await exportButton!.trigger('click')
    await flushPromises()

    // The backend deserializes the query into AuditLogQuery
    // (Option<Uuid>, Option<String>, Option<DateTime<Utc>>). The raw filter
    // object violates that contract twice: an empty-string param
    // (policy_id=) is a UUID parse error, and a bare yyyy-MM-dd date is a
    // DateTime parse error — either makes the whole request a 400. Assert
    // the params actually sent never do.
    for (const url of ['/admin/audit/membership/export', '/admin/audit/membership']) {
      const call = apiGet.mock.calls.find(c => c[0] === url)
      expect(call).toBeDefined()
      const cfg = call![1] as { params?: Record<string, unknown> } | undefined
      const params = cfg?.params ?? {}

      // No empty-string values (empty policy_id= fails Option<Uuid>).
      for (const value of Object.values(params)) {
        expect(String(value)).not.toBe('')
        expect(value).toBeDefined()
        expect(value).not.toBeNull()
      }

      // Dates must be full RFC3339 timestamps (yyyy-MM-dd alone fails
      // Option<DateTime<Utc>>).
      for (const key of ['from_date', 'to_date']) {
        if (params[key] !== undefined) {
          expect(String(params[key])).toMatch(
            /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2})$/
          )
        }
      }

      // The wire form (through the real serializer) contains no empty
      // values either.
      const qs = serializeQueryParams(params)
      expect(qs).not.toMatch(/=(?:&|$)/)
    }

    // List and export must send identical filters (shared param builder).
    const listParams = (
      apiGet.mock.calls.find(c => c[0] === '/admin/audit/membership')![1] as {
        params?: Record<string, unknown>
      }
    ).params
    const exportParams = (
      apiGet.mock.calls.find(c => c[0] === '/admin/audit/membership/export')![1] as {
        params?: Record<string, unknown>
      }
    ).params
    expect(exportParams).toEqual(listParams)

    wrapper.unmount()
  })
})
