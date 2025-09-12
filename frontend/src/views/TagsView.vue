<template>
  <div class="tags">
    <h1>Tags</h1>
    <ul v-if="tags.length">
      <li v-for="t in tags" :key="t">
        <router-link :to="{ name: 'home', query: { tag: t } }">{{ t }}</router-link>
      </li>
    </ul>
    <p v-else-if="error">{{ error }}</p>
    <p v-else>Loading...</p>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'

const tags = ref([])
const error = ref('')

onMounted(async () => {
  try {
    const res = await fetch('/api/tags')
    if (!res.ok) throw new Error('Request failed')
    tags.value = await res.json() || []
  } catch (e) {
    error.value = 'Failed to load'
  }
})
</script>
