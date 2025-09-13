import { defineStore } from 'pinia'
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
      const res = await fetch(`/api/articles${query ? `?${query}` : ''}`)
      if (res.ok) {
        const data = await res.json()
        this.articles = data.articles || []
      } else {
        this.articles = []
        throw new Error('Request failed')
      }
    },
    async fetchArticle(slug) {
      const res = await fetch(`/api/articles/${slug}`)
      if (res.ok) {
        this.currentArticle = await res.json()
      } else {
        this.currentArticle = null
        throw new Error('Request failed')
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
