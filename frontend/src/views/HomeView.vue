<template>
  <div class="home">
    <StateWrapper :loading="loading" :error="error" :data="store.articles">
      <ul>
        <li v-for="a in store.articles" :key="a.slug">
          <router-link :to="`/articles/${a.slug}`">{{ a.metadata.title }}</router-link>
        </li>
      </ul>
    </StateWrapper>
  </div>
</template>

<script setup>
import { ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { useMainStore } from '../store'
import StateWrapper from '../components/StateWrapper.vue'
import debounce from '../utils/debounce'

const store = useMainStore()
const error = ref('')
const loading = ref(false)
const route = useRoute()

const fetchArticles = async () => {
  loading.value = true
  try {
    const params = {}
    if (route.query.tag) params.tag = route.query.tag
    if (route.query.category) params.category = route.query.category
    await store.fetchArticles(params)
    error.value = ''
  } catch (e) {
    error.value = 'Failed to load'
  } finally {
    loading.value = false
  }
}
const debouncedFetchArticles = debounce(fetchArticles, 300)

watch(() => route.query, debouncedFetchArticles, { immediate: true })
</script>

<style scoped>
.home ul {
  list-style: none;
  padding: 0;
  margin: 0;
}
</style>
