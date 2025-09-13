<template>
  <div class="login-page">
    <div class="login-container">
      <div class="login-card">
        <div class="login-header">
          <h1>登录到 Scribe Blog</h1>
          <p>使用 GitHub 账户登录来发表评论或管理内容</p>
        </div>

        <div class="login-content">
          <StateWrapper :loading="loading" :error="error" :data="true">
            <div class="login-options">
              <button
                class="github-login-btn"
                @click="handleGitHubLogin"
                :disabled="loading"
              >
                <svg class="github-icon" viewBox="0 0 24 24" width="20" height="20">
                  <path fill="currentColor" d="M12,2A10,10 0 0,0 2,12C2,16.42 4.87,20.17 8.84,21.5C9.34,21.58 9.5,21.27 9.5,21C9.5,20.77 9.5,20.14 9.5,19.31C6.73,19.91 6.14,17.97 6.14,17.97C5.68,16.81 5.03,16.5 5.03,16.5C4.12,15.88 5.1,15.9 5.1,15.9C6.1,15.97 6.63,16.93 6.63,16.93C7.5,18.45 8.97,18 9.54,17.76C9.63,17.11 9.89,16.67 10.17,16.42C7.95,16.17 5.62,15.31 5.62,11.5C5.62,10.39 6,9.5 6.65,8.79C6.55,8.54 6.2,7.5 6.75,6.15C6.75,6.15 7.59,5.88 9.5,7.17C10.29,6.95 11.15,6.84 12,6.84C12.85,6.84 13.71,6.95 14.5,7.17C16.41,5.88 17.25,6.15 17.25,6.15C17.8,7.5 17.45,8.54 17.35,8.79C18,9.5 18.38,10.39 18.38,11.5C18.38,15.32 16.04,16.16 13.81,16.41C14.17,16.72 14.5,17.33 14.5,18.26C14.5,19.6 14.5,20.68 14.5,21C14.5,21.27 14.66,21.59 15.17,21.5C19.14,20.16 22,16.42 22,12A10,10 0 0,0 12,2Z" />
                </svg>
                使用 GitHub 登录
              </button>
            </div>

            <div class="login-info">
              <div class="info-section">
                <h3>👤 访客权限</h3>
                <ul>
                  <li>查看所有文章内容</li>
                  <li>发表和管理自己的评论</li>
                  <li>选择匿名发表评论</li>
                </ul>
              </div>

              <div class="info-section">
                <h3>✍️ 作者权限</h3>
                <ul>
                  <li>创建和编辑文章</li>
                  <li>管理所有评论</li>
                  <li>访问管理功能</li>
                </ul>
              </div>
            </div>
          </StateWrapper>
        </div>

        <div class="login-footer">
          <router-link to="/">← 返回首页</router-link>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { useRouter } from 'vue-router'
import StateWrapper from '../components/StateWrapper.vue'
import useAuth from '../composables/useAuth'

const router = useRouter()
const { loginWithGitHub, loading, error, isAuthenticated } = useAuth()

const handleGitHubLogin = () => {
  loginWithGitHub()
}

// 检查是否已经登录，如果是则重定向
if (isAuthenticated.value) {
  router.push('/')
}
</script>

<style scoped>
.login-page {
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  padding: 20px;
}

.login-container {
  width: 100%;
  max-width: 500px;
}

.login-card {
  background: white;
  border-radius: 12px;
  box-shadow: 0 20px 40px rgba(0, 0, 0, 0.1);
  overflow: hidden;
}

.login-header {
  padding: 40px 40px 20px;
  text-align: center;
  background: linear-gradient(135deg, #f5f7fa 0%, #c3cfe2 100%);
}

.login-header h1 {
  margin: 0 0 10px;
  color: #2d3748;
  font-size: 28px;
  font-weight: 600;
}

.login-header p {
  margin: 0;
  color: #718096;
  font-size: 16px;
}

.login-content {
  padding: 30px 40px;
}

.github-login-btn {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 16px 24px;
  background: #24292e;
  color: white;
  border: none;
  border-radius: 8px;
  font-size: 16px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s ease;
  margin-bottom: 30px;
}

.github-login-btn:hover:not(:disabled) {
  background: #1a1e22;
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(36, 41, 46, 0.3);
}

.github-login-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
  transform: none;
}

.github-icon {
  flex-shrink: 0;
}

.login-info {
  display: grid;
  gap: 20px;
}

.info-section {
  padding: 20px;
  background: #f7fafc;
  border-radius: 8px;
  border-left: 4px solid #4299e1;
}

.info-section h3 {
  margin: 0 0 12px;
  color: #2d3748;
  font-size: 16px;
  font-weight: 600;
}

.info-section ul {
  margin: 0;
  padding-left: 20px;
  color: #4a5568;
}

.info-section li {
  margin-bottom: 6px;
  font-size: 14px;
}

.login-footer {
  padding: 20px 40px;
  border-top: 1px solid #e2e8f0;
  text-align: center;
}

.login-footer a {
  color: #4299e1;
  text-decoration: none;
  font-weight: 500;
}

.login-footer a:hover {
  text-decoration: underline;
}

@media (max-width: 768px) {
  .login-page {
    padding: 10px;
  }

  .login-header,
  .login-content,
  .login-footer {
    padding-left: 20px;
    padding-right: 20px;
  }

  .login-header h1 {
    font-size: 24px;
  }
}
</style>