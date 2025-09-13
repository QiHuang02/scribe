import { ref } from 'vue'

export default function useApi(initialValue = null, { retries = 3, retryDelay = 500 } = {}) {
  const data = ref(initialValue)
  const error = ref('')
  const loading = ref(false)

  const request = async (url, options, config = {}) => {
    loading.value = true
    error.value = ''

    const maxRetries = config.retries ?? retries
    const baseDelay = config.retryDelay ?? retryDelay
    let attempt = 0
    let delay = baseDelay
    let lastError

    while (attempt < maxRetries) {
      try {
        const res = await fetch(url, options)
        if (!res.ok) {
          let message = 'Request failed'
          try {
            const errData = await res.json()
            message = errData.message || message
          } catch (_) {
            // ignore parse error
          }
          throw new Error(message)
        }
        data.value = (await res.json()) || initialValue
        loading.value = false
        return
      } catch (e) {
        lastError = e
        attempt++
        if (attempt < maxRetries) {
          await new Promise(resolve => setTimeout(resolve, delay))
          delay *= 2
        }
      }
    }

    error.value = lastError?.message || 'Failed to load'
    loading.value = false
  }

  return { data, error, loading, request }
}
