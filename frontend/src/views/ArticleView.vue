<template>
  <div class="article">
    <template v-if="store.currentArticle">
      <h1>{{ store.currentArticle.metadata.title }}</h1>
      <div v-html="sanitizedHtml"></div>
      <div v-if="versions.length">
        <h2>Versions</h2>
        <ul>
          <li v-for="v in versions" :key="v.version">
            {{ new Date(v.timestamp).toLocaleString() }}
            <router-link :to="`/articles/${route.params.slug}/versions/${v.version}`">Preview</router-link>
            <button v-if="isAuthorized" @click="restore(v.version)">Restore</button>
          </li>
        </ul>
      </div>
    </template>
    <p v-else-if="error">{{ error }}</p>
    <p v-else>Loading...</p>
  </div>
</template>

<script setup>
import { ref, onMounted, computed } from 'vue'
import { useRoute } from 'vue-router'
import DOMPurify from 'dompurify'
import { md } from '../utils/markdown'
import { useMainStore } from '../store'

const route = useRoute()
const store = useMainStore()
const versions = ref([])
const error = ref('')
const isAuthorized = computed(() => store.user?.isAdmin)

const sanitizedHtml = computed(() => {
  if (!store.currentArticle) return ''
  return DOMPurify.sanitize(md.render(store.currentArticle.content || ''))
})

async function load() {
  try {
    await store.fetchArticle(route.params.slug)
  } catch (e) {
    error.value = 'Failed to load'
  }
}

async function loadVersions() {
  try {
    const res = await fetch(`/api/articles/${route.params.slug}/versions`)
    if (!res.ok) throw new Error('Request failed')
    versions.value = await res.json()
  } catch (e) {
    // ignore
  }
}

async function restore(version) {
  try {
    const token = store.token
    await fetch(`/api/articles/${route.params.slug}/versions/${version}/restore`, {
      method: 'POST',
      headers: {
        ...(token ? { Authorization: `Bearer ${token}` } : {})
      }
    })
    await load()
    await loadVersions()
  } catch (e) {
    // ignore
  }
}

onMounted(() => {
  load()
  loadVersions()
})
</script>

<style scoped>
.article ul {
  list-style: none;
  padding: 0;
  margin: 0;
}
</style>
