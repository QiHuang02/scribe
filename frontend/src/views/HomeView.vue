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
import { ref, onMounted } from 'vue'

const articles = ref([])
const error = ref('')

onMounted(async () => {
  try {
    const res = await fetch('/api/articles')
    if (!res.ok) throw new Error('Request failed')
    const data = await res.json()
    articles.value = data.articles || []
  } catch (e) {
    error.value = 'Failed to load'
  }
})
</script>
