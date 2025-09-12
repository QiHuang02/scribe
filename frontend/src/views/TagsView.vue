<template>
  <div class="tags">
    <ul v-if="!loading && tags.length">
      <li v-for="t in tags" :key="t">
        <router-link :to="{ name: 'home', query: { tag: t } }">{{ t }}</router-link>
      </li>
    </ul>
    <p v-else-if="error">{{ error }}</p>
    <p v-else>Loading...</p>
  </div>
</template>

<script setup>
import { onMounted } from 'vue'
import useApi from '../composables/useApi'

const { data: tags, error, loading, request } = useApi([])

onMounted(() => {
  request('/api/tags')
})
</script>

<style scoped>
.tags ul {
  list-style: none;
  padding: 0;
  margin: 0;
}
</style>
