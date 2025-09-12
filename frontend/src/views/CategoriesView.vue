<template>
  <div class="categories">
    <ul v-if="!loading && categories.length">
      <li v-for="c in categories" :key="c">
        <router-link :to="{ name: 'home', query: { category: c } }">{{ c }}</router-link>
      </li>
    </ul>
    <p v-else-if="error">{{ error }}</p>
    <p v-else>Loading...</p>
  </div>
</template>

<script setup>
import { onMounted } from 'vue'
import useApi from '../composables/useApi'

const { data: categories, error, loading, request } = useApi([])

onMounted(() => {
  request('/api/categories')
})
</script>

<style scoped>
.categories ul {
  list-style: none;
  padding: 0;
  margin: 0;
}
</style>
