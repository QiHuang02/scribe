<template>
  <div id="app">
    <nav class="main-nav">
      <div class="nav-brand">
        <router-link to="/" class="brand-link">Scribe Blog</router-link>
      </div>

      <div class="nav-links">
        <router-link to="/">Articles</router-link>
        <router-link to="/notes">Notes</router-link>
        <router-link to="/categories">Categories</router-link>
        <router-link to="/tags">Tags</router-link>
        <router-link to="/search">Search</router-link>
        <router-link to="/about">About</router-link>
      </div>

      <div class="nav-user">
        <UserMenu />
      </div>
    </nav>

    <main class="main-content">
      <router-view/>
    </main>
  </div>
</template>

<script setup>
import { onMounted } from 'vue'
import UserMenu from './components/UserMenu.vue'
import { useMainStore } from './store'

const store = useMainStore()

onMounted(() => {
  // 初始化认证状态
  store.initializeAuth()
})
</script>

<style>
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

#app {
  font-family: Avenir, Helvetica, Arial, sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  color: #2c3e50;
  min-height: 100vh;
}

.main-nav {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 24px;
  background: white;
  border-bottom: 1px solid #e2e8f0;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
}

.nav-brand .brand-link {
  font-size: 20px;
  font-weight: 700;
  color: #2d3748;
  text-decoration: none;
}

.nav-links {
  display: flex;
  align-items: center;
  gap: 24px;
}

.nav-links a {
  color: #4a5568;
  text-decoration: none;
  font-weight: 500;
  font-size: 14px;
  padding: 8px 12px;
  border-radius: 6px;
  transition: all 0.2s ease;
}

.nav-links a:hover {
  color: #2d3748;
  background: #f7fafc;
}

.nav-links a.router-link-exact-active {
  color: #4299e1;
  background: #ebf8ff;
}

.main-content {
  padding: 24px;
  max-width: 1200px;
  margin: 0 auto;
}

@media (max-width: 768px) {
  .main-nav {
    padding: 12px 16px;
  }

  .nav-links {
    gap: 16px;
  }

  .nav-links a {
    font-size: 13px;
    padding: 6px 8px;
  }

  .main-content {
    padding: 16px;
  }
}

@media (max-width: 640px) {
  .nav-links {
    gap: 8px;
  }

  .nav-links a {
    font-size: 12px;
    padding: 4px 6px;
  }
}
</style>
