<template>
  <div class="note">
    <template v-if="note">
      <h1>{{ note.metadata.title }}</h1>
      <div v-html="sanitizedHtml"></div>
    </template>
    <p v-else-if="error">{{ error }}</p>
    <p v-else>Loading...</p>
  </div>
</template>

<script setup>
import { ref, onMounted, computed } from 'vue'
import { useRoute } from 'vue-router'
import DOMPurify from 'dompurify'
import { md } from '../utils/markdown'

const route = useRoute()
const note = ref(null)
const error = ref('')

const sanitizedHtml = computed(() => {
  if (!note.value) return ''
  return DOMPurify.sanitize(md.render(note.value.content || ''))
})

async function load() {
  try {
    const slug = route.params.slug
    const res = await fetch(`/api/notes/${slug}`)
    if (!res.ok) throw new Error('Request failed')
    note.value = await res.json()
  } catch (e) {
    error.value = 'Failed to load'
  }
}

onMounted(load)
</script>

<style scoped>
.note ul {
  list-style: none;
  padding: 0;
  margin: 0;
}
</style>
