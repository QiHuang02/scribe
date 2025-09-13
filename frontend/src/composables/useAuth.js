import { ref, computed } from 'vue'
import useApi from './useApi'

const currentUser = ref(null)
const isAuthenticated = computed(() => !!currentUser.value)
const isAuthor = computed(() => currentUser.value?.is_author || false)
const isVisitor = computed(() => currentUser.value && !currentUser.value.is_author)

export default function useAuth() {
  const { data, error, loading, request } = useApi()

  // 获取当前用户信息
  const getCurrentUser = async () => {
    try {
      await request('/api/auth/me')
      if (!error.value) {
        currentUser.value = data.value
        return currentUser.value
      }
      currentUser.value = null
      return null
    } catch (e) {
      currentUser.value = null
      return null
    }
  }

  // GitHub 登录
  const loginWithGitHub = () => {
    window.location.href = '/api/auth/github/login'
  }

  // 登出
  const logout = () => {
    currentUser.value = null
    // 清除所有 cookies (简单方式)
    document.cookie.split(";").forEach((c) => {
      document.cookie = c.replace(/^ +/, "").replace(/=.*/, "=;expires=" + new Date().toUTCString() + ";path=/")
    })
    // 重定向到首页
    window.location.href = '/'
  }

  // 设置用户信息 (用于登录回调后)
  const setUser = (user) => {
    currentUser.value = user
  }

  return {
    // 状态
    currentUser: computed(() => currentUser.value),
    isAuthenticated,
    isAuthor,
    isVisitor,
    loading,
    error,

    // 方法
    getCurrentUser,
    loginWithGitHub,
    logout,
    setUser
  }
}