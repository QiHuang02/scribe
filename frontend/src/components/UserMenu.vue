<template>
  <div class="user-menu">
    <!-- 未登录状态 -->
    <div v-if="!isAuthenticated" class="login-section">
      <router-link to="/login" class="login-btn">
        登录
      </router-link>
    </div>

    <!-- 已登录状态 -->
    <div v-else class="user-section">
      <div class="user-dropdown" @click="toggleDropdown" ref="dropdownRef">
        <div class="user-info">
          <img
            :src="user?.avatar || `https://github.com/${user?.github_login}.png`"
            :alt="user?.display_name"
            class="user-avatar"
          />
          <span class="user-name">{{ user?.display_name }}</span>
          <span class="dropdown-arrow" :class="{ open: showDropdown }">▼</span>
        </div>

        <div v-if="showDropdown" class="dropdown-menu">
          <div class="user-role">
            <span class="role-badge" :class="roleClass">
              {{ isAuthor ? '✍️ 作者' : '👤 访客' }}
            </span>
          </div>

          <div class="dropdown-divider"></div>

          <div class="dropdown-items">
            <Suspense v-if="isAuthor && authorMenuComponent">
              <template #default>
                <component :is="authorMenuComponent" />
              </template>
              <template #fallback>
                <div class="dropdown-item">加载中...</div>
              </template>
            </Suspense>

            <div class="dropdown-item">
              <a
                :href="`https://github.com/${user?.github_login}`"
                target="_blank"
                class="dropdown-link"
              >
                🔗 GitHub 主页
              </a>
            </div>

            <div class="dropdown-divider"></div>

            <div class="dropdown-item">
              <button @click="handleLogout" class="dropdown-button logout-btn">
                🚪 登出
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted, onUnmounted, computed, watch } from 'vue'
import { useMainStore } from '../store'
import useAuth from '../composables/useAuth'

const store = useMainStore()
const { currentUser, isAuthenticated, isAuthor, logout } = useAuth()

const showDropdown = ref(false)
const dropdownRef = ref(null)

const user = currentUser
const roleClass = computed(() => (isAuthor.value ? 'author' : 'visitor'))

// Lazily loaded author-specific menu component
const authorMenuComponent = ref(null)
watch(
  isAuthor,
  async (val) => {
    if (val && !authorMenuComponent.value) {
      authorMenuComponent.value = (await import('./AuthorMenu.vue')).default
    }
  },
  { immediate: true }
)

const toggleDropdown = () => {
  showDropdown.value = !showDropdown.value
}

const handleLogout = async () => {
  showDropdown.value = false
  await logout()
}

// 点击外部关闭下拉菜单
const handleClickOutside = (event) => {
  if (dropdownRef.value && !dropdownRef.value.contains(event.target)) {
    showDropdown.value = false
  }
}

onMounted(() => {
  document.addEventListener('click', handleClickOutside)
  // 初始化认证状态
  store.initializeAuth()
})

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside)
})
</script>

<style scoped>
.user-menu {
  position: relative;
}

.login-btn {
  display: inline-flex;
  align-items: center;
  padding: 8px 16px;
  background: #4299e1;
  color: white;
  text-decoration: none;
  border-radius: 6px;
  font-size: 14px;
  font-weight: 500;
  transition: background-color 0.2s ease;
}

.login-btn:hover {
  background: #3182ce;
}

.user-dropdown {
  position: relative;
  cursor: pointer;
}

.user-info {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border-radius: 8px;
  transition: background-color 0.2s ease;
}

.user-info:hover {
  background: #f7fafc;
}

.user-avatar {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  border: 2px solid #e2e8f0;
}

.user-name {
  font-size: 14px;
  font-weight: 500;
  color: #2d3748;
}

a.user-name {
  text-decoration: none;
}

.dropdown-arrow {
  font-size: 10px;
  color: #a0aec0;
  transition: transform 0.2s ease;
}

.dropdown-arrow.open {
  transform: rotate(180deg);
}

.dropdown-menu {
  position: absolute;
  top: 100%;
  right: 0;
  z-index: 50;
  min-width: 200px;
  background: white;
  border: 1px solid #e2e8f0;
  border-radius: 8px;
  box-shadow: 0 10px 25px rgba(0, 0, 0, 0.1);
  padding: 8px 0;
  margin-top: 4px;
}

.user-role {
  padding: 12px 16px;
  text-align: center;
}

.role-badge {
  display: inline-block;
  padding: 4px 12px;
  border-radius: 20px;
  font-size: 12px;
  font-weight: 500;
}

.role-badge.author {
  background: #ffd6e7;
  color: #b83280;
}

.role-badge.visitor {
  background: #e6fffa;
  color: #319795;
}

.dropdown-divider {
  height: 1px;
  background: #e2e8f0;
  margin: 8px 0;
}

.dropdown-item {
  padding: 0 8px;
}

.dropdown-link,
.dropdown-button {
  display: flex;
  align-items: center;
  width: 100%;
  padding: 8px 12px;
  color: #4a5568;
  text-decoration: none;
  font-size: 14px;
  border-radius: 4px;
  transition: background-color 0.2s ease;
  border: none;
  background: none;
  cursor: pointer;
  text-align: left;
}

.dropdown-link:hover,
.dropdown-button:hover {
  background: #f7fafc;
}

.logout-btn:hover {
  background: #fed7d7;
  color: #e53e3e;
}

@media (max-width: 768px) {
  .user-name {
    display: none;
  }

  .dropdown-menu {
    right: -8px;
  }
}
</style>
