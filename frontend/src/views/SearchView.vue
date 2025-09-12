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

const query = ref('')
const results = ref([])
const error = ref('')
const popular = ref([])
const loading = ref(false)

async function performSearch() {
  if (!query.value.trim()) {
    results.value = []
    return
  }
  loading.value = true
  try {
    const res = await fetch(`/api/search?q=${encodeURIComponent(query.value)}`)
    if (!res.ok) throw new Error('Request failed')
    const data = await res.json()
    results.value = data.results || []
    error.value = ''
  } catch (e) {
    error.value = 'Failed to load'
    results.value = []
  } finally {
    loading.value = false
  }
}

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
