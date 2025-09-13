import { defineStore } from 'pinia'
import useApi from '../composables/useApi'
import {
  getToken as loadToken,
  setToken as saveToken,
  clearToken,
  isAdmin,
  setIsAdmin
} from '../utils/storage'

export const useMainStore = defineStore('main', {
  state: () => ({
    articles: [],
    currentArticle: null,
    user: { isAdmin: isAdmin() },
    token: loadToken() || ''
  }),
  actions: {
    async fetchArticles(params = {}) {
      const query = new URLSearchParams(params).toString()
      const api = useApi({ articles: [] })
      await api.request(`/api/articles${query ? `?${query}` : ''}`)
      if (!api.error.value) {
        this.articles = api.data.value.articles || []
      } else {
        this.articles = []
        throw new Error(api.error.value)
      }
    },
    async fetchArticle(slug) {
      const api = useApi()
      await api.request(`/api/articles/${slug}`)
      if (!api.error.value) {
        this.currentArticle = api.data.value
      } else {
        this.currentArticle = null
        throw new Error(api.error.value)
      }
    },
    setUser(user) {
      this.user = user
      setIsAdmin(user?.isAdmin)
    },
    setToken(token) {
      this.token = token
      if (token) {
        saveToken(token)
      } else {
        clearToken()
      }
    }
  }
})
