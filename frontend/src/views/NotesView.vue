<template>
  <div class="notes">
    <StateWrapper :loading="loading" :error="error" :data="notes">
      <ul>
        <li v-for="n in notes" :key="n.slug">
          <router-link :to="`/notes/${n.slug}`">{{ n.metadata.title }}</router-link>
        </li>
      </ul>
    </StateWrapper>
  </div>
</template>

<script setup>
import { ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import StateWrapper from '../components/StateWrapper.vue'

const notes = ref([])
const error = ref('')
const loading = ref(false)
const route = useRoute()

const fetchNotes = async () => {
  loading.value = true
  try {
    const params = new URLSearchParams()
    if (route.query.tag) params.set('tag', route.query.tag)
    if (route.query.category) params.set('category', route.query.category)
    const query = params.toString()
    const res = await fetch(`/api/notes${query ? `?${query}` : ''}`)
    if (!res.ok) throw new Error('Request failed')
    const data = await res.json()
    notes.value = data.articles || []
    error.value = ''
  } catch (e) {
    error.value = 'Failed to load'
    notes.value = []
  } finally {
    loading.value = false
  }
}

watch(() => route.query, fetchNotes, { immediate: true })
</script>

<style scoped>
.notes ul {
  list-style: none;
  padding: 0;
  margin: 0;
}
</style>
