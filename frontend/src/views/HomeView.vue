<template>
  <div class="home">
    <ul v-if="store.articles.length">
      <li v-for="a in store.articles" :key="a.slug">
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
import { useMainStore } from '../store'

const store = useMainStore()
const error = ref('')
const route = useRoute()

const fetchArticles = async () => {
  try {
    const params = {}
    if (route.query.tag) params.tag = route.query.tag
    if (route.query.category) params.category = route.query.category
    await store.fetchArticles(params)
    error.value = ''
  } catch (e) {
    error.value = 'Failed to load'
  }
}

watch(() => route.query, fetchArticles, { immediate: true })
</script>

<style scoped>
.home ul {
  list-style: none;
  padding: 0;
  margin: 0;
}
</style>
