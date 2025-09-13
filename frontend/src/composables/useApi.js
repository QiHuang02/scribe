import { ref } from 'vue'

export default function useApi(initialValue = null) {
  const data = ref(initialValue)
  const error = ref('')
  const loading = ref(false)

  const request = async (url, options) => {
    loading.value = true
    error.value = ''
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
    } catch (e) {
      error.value = e.message || 'Failed to load'
    } finally {
      loading.value = false
    }
  }

  return { data, error, loading, request }
}
