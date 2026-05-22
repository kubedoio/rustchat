/**
 * HTTP Client module
 * Fetch-based transport layer replacing Axios
 */

export { HttpClient } from './HttpClient'
export type { HttpClientConfig, RequestConfig, HttpResponse } from './HttpClient'
export {
  TimeoutError,
  AbortError,
  HttpError,
  isTimeoutError,
  isAbortError,
  isHttpError,
} from './errors'
export { serializeQueryParams, buildURL } from './querySerializer'
export { uploadWithProgress, type UploadConfig, type UploadProgress } from './uploadWithProgress'
