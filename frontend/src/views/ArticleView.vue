<template>
  <div class="article">
    <h1 v-if="article">{{ article.metadata.title }}</h1>
    <pre v-if="article">{{ article.content }}</pre>
    <p v-else-if="error">{{ error }}</p>
    <p v-else>Loading...</p>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { useRoute } from 'vue-router'

const route = useRoute()
const article = ref(null)
const error = ref('')

async function load() {
  try {
    const res = await fetch(`/api/articles/${route.params.slug}`)
    if (!res.ok) throw new Error('Request failed')
    article.value = await res.json()
  } catch (e) {
    error.value = 'Failed to load'
  }
}

onMounted(load)
</script>
