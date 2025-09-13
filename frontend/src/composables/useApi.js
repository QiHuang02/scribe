import { ref } from 'vue'

// Map raw error details to concise user-facing messages
const getErrorMessage = (status, message, error) => {
  if (typeof navigator !== 'undefined' && !navigator.onLine) {
    return 'Network unreachable'
  }
  if (status === 401) {
    return 'Unauthorized'
  }
  if (error?.message && error.message.toLowerCase().includes('failed to fetch')) {
    return 'Network unreachable'
  }
  return message || 'Failed to load'
}

export default function useApi(initialValue = null, { retries = 3, retryDelay = 500 } = {}) {
  const data = ref(initialValue)
  const error = ref('')
  const status = ref(null)
  const loading = ref(false)

  let controller

  const cancel = () => {
    if (controller) {
      controller.abort()
      controller = null
    }
  }

  const request = async (url, options = {}, config = {}) => {
    cancel()
    controller = new AbortController()
    const currentController = controller

    loading.value = true
    error.value = ''
    status.value = null

    const maxRetries = config.retries ?? retries
    const baseDelay = config.retryDelay ?? retryDelay
    let attempt = 0
    let delay = baseDelay
    let lastError

    while (attempt < maxRetries) {
      try {
        const res = await fetch(url, { ...options, signal: controller.signal })
        if (!res.ok) {
          let message = 'Request failed'
          try {
            const errData = await res.json()
            message = errData.message || message
          } catch (_) {
            // ignore parse error
          }
          const err = new Error(message)
          err.status = res.status
          throw err
        }
        data.value = (await res.json()) || initialValue
        loading.value = false
        return
      } catch (e) {
        if (
          e.name === 'AbortError' ||
          (typeof DOMException !== 'undefined' && e instanceof DOMException)
        ) {
          if (controller === null) {
            loading.value = false
          }
          return
        }
        if (currentController !== controller) {
          return
        }
        lastError = e
        attempt++
        if (attempt < maxRetries) {
          await new Promise(resolve => setTimeout(resolve, delay))
          delay *= 2
        }
      }
    }

    console.error(lastError)
    status.value = lastError?.status || null
    error.value = getErrorMessage(status.value, lastError?.message, lastError)
    loading.value = false
  }

  return { data, error, status, loading, request, cancel }
}
