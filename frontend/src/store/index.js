import { defineStore } from 'pinia'
import useApi from '../composables/useApi'
import useAuth from '../composables/useAuth'

export const useMainStore = defineStore('main', {
  state: () => ({
    articles: [],
    currentArticle: null,
  }),

  getters: {
    // 从 useAuth composable 获取用户状态
    user: () => {
      const { currentUser } = useAuth()
      return currentUser.value
    },
    isAuthenticated: () => {
      const { isAuthenticated } = useAuth()
      return isAuthenticated.value
    },
    isAuthor: () => {
      const { isAuthor } = useAuth()
      return isAuthor.value
    },
    isVisitor: () => {
      const { isVisitor } = useAuth()
      return isVisitor.value
    }
  },

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

    // 认证相关方法
    async initializeAuth() {
      const { getCurrentUser } = useAuth()
      await getCurrentUser()
    },

    async loginWithGitHub() {
      const { loginWithGitHub } = useAuth()
      loginWithGitHub()
    },

    async logout() {
      const { logout } = useAuth()
      logout()
    }
  }
})
