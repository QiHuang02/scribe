<template>
  <div class="home">
    <h1>{{ message }}</h1>
    <el-button type="primary">Primary Button</el-button>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'

const message = ref('Loading...')

onMounted(async () => {
  try {
    const res = await fetch('/')
    message.value = await res.text()
  } catch (e) {
    message.value = 'Failed to load'
  }
})
</script>
