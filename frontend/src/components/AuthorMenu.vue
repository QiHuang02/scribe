<template>
  <div class="dropdown-item">
    <router-link to="/articles/new" class="dropdown-link">
      ✏️ 写文章
    </router-link>
  </div>
</template>

<script setup>
// This component could perform author-specific data fetching.
// It is lazy-loaded to avoid unnecessary network requests for non-author users.
</script>

<style scoped>
.dropdown-item {
  padding: 0 8px;
}

.dropdown-link {
  display: flex;
  align-items: center;
  width: 100%;
  padding: 8px 12px;
  color: #4a5568;
  text-decoration: none;
  font-size: 14px;
  border-radius: 4px;
  transition: background-color 0.2s ease;
}

.dropdown-link:hover {
  background: #f7fafc;
}
</style>
