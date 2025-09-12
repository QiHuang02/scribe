<template>
  <div class="article">
    <h1 v-if="article">{{ article.metadata.title }}</h1>
    <pre v-if="article">{{ article.content }}</pre>
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
    <p v-else-if="error">{{ error }}</p>
    <p v-else>Loading...</p>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { useRoute } from 'vue-router'

const route = useRoute()
const article = ref(null)
const versions = ref([])
const error = ref('')
const isAuthorized = localStorage.getItem('isAdmin') === 'true'

async function load() {
  try {
    const res = await fetch(`/api/articles/${route.params.slug}`)
    if (!res.ok) throw new Error('Request failed')
    article.value = await res.json()
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
    await fetch(`/api/articles/${route.params.slug}/versions/${version}/restore`, { method: 'POST' })
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
