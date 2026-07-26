/**
 * A fetch wrapper that enforces a configurable timeout via AbortController.
 * When the caller also provides an external signal (e.g. for user cancellation),
 * both signals are merged so that either one firing will abort the request.
 *
 * Default timeout: 30 seconds.
 */
export async function fetchWithTimeout(
  input: RequestInfo | URL,
  init?: RequestInit & { timeout?: number },
): Promise<Response> {
  const { timeout = 30000, ...rest } = init || {}

  const controller = new AbortController()
  const timer = setTimeout(() => controller.abort(), timeout)

  // If the caller provided an external AbortSignal, relay its abort to ours.
  const externalSignal = rest.signal
  if (externalSignal) {
    if (externalSignal.aborted) {
      controller.abort()
    } else {
      externalSignal.addEventListener('abort', () => controller.abort())
    }
  }

  try {
    const response = await fetch(input, {
      ...rest,
      signal: controller.signal,
    })
    return response
  } finally {
    clearTimeout(timer)
  }
}
