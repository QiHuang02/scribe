<template>
  <el-skeleton v-if="loading" animated />
  <el-empty v-else-if="error" :description="error" />
  <el-empty v-else-if="!hasContent" />
  <slot v-else />
</template>

<script setup>
import { computed } from 'vue'

// eslint-disable-next-line no-undef
const props = defineProps({
  loading: Boolean,
  error: String,
  data: { type: [Array, Object], default: null }
})

const hasContent = computed(() => {
  if (Array.isArray(props.data)) return props.data.length > 0
  return !!props.data
})
</script>
