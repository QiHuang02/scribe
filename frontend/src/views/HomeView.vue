<template>
  <div class="home">
    <h1>Articles</h1>
    <ul v-if="articles.length">
      <li v-for="a in articles" :key="a.slug">
        <router-link :to="`/articles/${a.slug}`">{{ a.metadata.title }}</router-link>
      </li>
    </ul>
    <p v-else-if="error">{{ error }}</p>
    <p v-else>Loading...</p>
  </div>
</template>

<script setup>
import { ref, watch } from 'vue'
import { useRoute } from 'vue-router'

const articles = ref([])
const error = ref('')
const route = useRoute()

const fetchArticles = async () => {
  try {
    const params = new URLSearchParams()
    if (route.query.tag) params.set('tag', route.query.tag)
    if (route.query.category) params.set('category', route.query.category)
    const query = params.toString()
    const res = await fetch(`/api/articles${query ? `?${query}` : ''}`)
    if (!res.ok) throw new Error('Request failed')
    const data = await res.json()
    articles.value = data.articles || []
    error.value = ''
  } catch (e) {
    error.value = 'Failed to load'
    articles.value = []
  }
}

watch(() => route.query, fetchArticles, { immediate: true })
</script>
