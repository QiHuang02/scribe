<template>
  <div class="article-version">
    <h1 v-if="versionData">Version {{ version }} of {{ slug }}</h1>
    <div v-if="versionData" v-html="sanitizedHtml"></div>
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
const slug = route.params.slug
const version = route.params.version
const versionData = ref(null)
const error = ref('')

const sanitizedHtml = computed(() => {
  if (!versionData.value) return ''
  return DOMPurify.sanitize(md.render(versionData.value.content || ''))
})

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
