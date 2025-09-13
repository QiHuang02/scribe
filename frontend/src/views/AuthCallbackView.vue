<template>
  <div class="auth-callback">
    <div class="callback-container">
      <StateWrapper :loading="loading" :error="error" :data="!loading && !error">
        <div class="success-message">
          <div class="success-icon">✅</div>
          <h2>登录成功！</h2>
          <p v-if="user">欢迎回来，{{ user.display_name }}！</p>
          <p class="redirect-info">正在跳转到首页...</p>
        </div>
      </StateWrapper>
    </div>
  </div>
</template>

<script setup>
import { onMounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import StateWrapper from '../components/StateWrapper.vue'
import useAuth from '../composables/useAuth'

const router = useRouter()
const route = useRoute()
const { getCurrentUser, currentUser, loading, error } = useAuth()

onMounted(async () => {
  try {
    // 检查URL中是否有错误参数
    if (route.query.error) {
      throw new Error(route.query.error_description || '登录失败')
    }

    // 获取当前用户信息
    await getCurrentUser()

    // 延迟一下再跳转，让用户看到成功消息
    setTimeout(() => {
      router.push('/')
    }, 2000)
  } catch (e) {
    console.error('Auth callback error:', e)
    // 如果出错，延迟后跳转到登录页
    setTimeout(() => {
      router.push('/login')
    }, 3000)
  }
})

const user = currentUser
</script>

<style scoped>
.auth-callback {
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  padding: 20px;
}

.callback-container {
  width: 100%;
  max-width: 400px;
}

.success-message {
  background: white;
  border-radius: 12px;
  padding: 40px;
  text-align: center;
  box-shadow: 0 20px 40px rgba(0, 0, 0, 0.1);
}

.success-icon {
  font-size: 48px;
  margin-bottom: 20px;
}

.success-message h2 {
  margin: 0 0 16px;
  color: #2d3748;
  font-size: 24px;
  font-weight: 600;
}

.success-message p {
  margin: 8px 0;
  color: #4a5568;
  font-size: 16px;
}

.redirect-info {
  color: #718096 !important;
  font-size: 14px !important;
  margin-top: 20px !important;
}
</style>