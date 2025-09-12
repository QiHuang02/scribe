<template>
  <div class="categories">
    <h1>Categories</h1>
    <ul v-if="categories.length">
      <li v-for="c in categories" :key="c">
        <router-link :to="{ name: 'home', query: { category: c } }">{{ c }}</router-link>
      </li>
    </ul>
    <p v-else-if="error">{{ error }}</p>
    <p v-else>Loading...</p>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'

const categories = ref([])
const error = ref('')

onMounted(async () => {
  try {
    const res = await fetch('/api/categories')
    if (!res.ok) throw new Error('Request failed')
    categories.value = await res.json() || []
  } catch (e) {
    error.value = 'Failed to load'
  }
})
</script>
