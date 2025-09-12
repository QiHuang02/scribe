<template>
  <div class="search">
    <h1>Search</h1>
    <input
      v-model="query"
      @keyup.enter="performSearch"
      placeholder="Search articles"
    />
    <button @click="performSearch">Search</button>

    <div v-if="results.length">
      <ul>
        <li v-for="r in results" :key="r.slug">
          <router-link :to="`/articles/${r.slug}`">{{ r.title }}</router-link>
        </li>
      </ul>
    </div>
    <p v-else-if="error">{{ error }}</p>
    <p v-else-if="query && !loading">No results found</p>
    <p v-else-if="loading">Loading...</p>

    <div v-if="!query && popular.length">
      <h2>Trending</h2>
      <ul>
        <li v-for="p in popular" :key="p.query">
          <a href="#" @click.prevent="setQuery(p.query)">{{ p.query }}</a>
        </li>
      </ul>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'

// simple debounce helper
function debounce(fn, wait = 300) {
  let timeout
  return (...args) => {
    clearTimeout(timeout)
    timeout = setTimeout(() => fn(...args), wait)
  }
}

const query = ref('')
const results = ref([])
const error = ref('')
const popular = ref([])
const loading = ref(false)
let controller

async function doSearch() {
  const q = query.value.trim()
  if (!q) {
    if (controller) controller.abort()
    controller = null
    results.value = []
    error.value = ''
    loading.value = false
    return
  }

  if (controller) controller.abort()
  const localController = new AbortController()
  controller = localController
  loading.value = true
  try {
    const res = await fetch(`/api/search?q=${encodeURIComponent(q)}`, {
      signal: localController.signal,
    })
    if (!res.ok) throw new Error(`Request failed with status ${res.status}`)
    const data = await res.json()
    results.value = data.results || []
    error.value = ''
  } catch (e) {
    if (e.name === 'AbortError') return
    error.value = `Failed to load: ${e.message}`
    results.value = []
  } finally {
    if (controller === localController) {
      loading.value = false
    }
  }
}

const performSearch = debounce(doSearch, 300)

function setQuery(q) {
  query.value = q
  performSearch()
}

onMounted(async () => {
  try {
    const res = await fetch('/api/search/popular')
    if (res.ok) {
      const data = await res.json()
      popular.value = data.searches || []
    }
  } catch (e) {
    // ignore errors for optional popular searches
  }
})
</script>
