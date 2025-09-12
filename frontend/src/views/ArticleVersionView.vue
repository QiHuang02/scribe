<template>
  <div class="article-version">
    <h1 v-if="versionData">Version {{ version }} of {{ slug }}</h1>
    <pre v-if="versionData">{{ versionData.content }}</pre>
    <p v-else-if="error">{{ error }}</p>
    <p v-else>Loading...</p>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { useRoute } from 'vue-router'

const route = useRoute()
const slug = route.params.slug
const version = route.params.version
const versionData = ref(null)
const error = ref('')

async function load() {
  try {
    const res = await fetch(`/api/articles/${slug}/versions/${version}`)
    if (!res.ok) throw new Error('Request failed')
    versionData.value = await res.json()
  } catch (e) {
    error.value = 'Failed to load'
  }
}

onMounted(load)
</script>
