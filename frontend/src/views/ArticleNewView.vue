<template>
  <el-card class="article-new">
    <el-form ref="formRef" :model="form" :rules="rules" label-width="80px">
      <el-form-item label="标题" prop="title">
        <el-input v-model="form.title" placeholder="请输入标题" />
      </el-form-item>

      <el-form-item label="描述">
        <el-input type="textarea" v-model="form.description" />
      </el-form-item>

      <el-form-item label="标签">
        <el-select
          v-model="form.tags"
          multiple
          filterable
          allow-create
          default-first-option
          placeholder="选择或输入标签"
        >
          <el-option
            v-for="tag in tagOptions"
            :key="tag"
            :label="tag"
            :value="tag"
          />
        </el-select>
      </el-form-item>

      <el-form-item label="分类">
        <el-select
          v-model="form.category"
          filterable
          allow-create
          default-first-option
          placeholder="选择或输入分类"
        >
          <el-option
            v-for="cat in categoryOptions"
            :key="cat"
            :label="cat"
            :value="cat"
          />
        </el-select>
      </el-form-item>

      <el-form-item label="内容" prop="content">
        <el-button size="small" @click="isPreview = !isPreview" style="margin-bottom: 10px">
          {{ isPreview ? '编辑' : '预览' }}
        </el-button>
        <textarea
          v-if="!isPreview"
          v-model="form.content"
          class="editor"
        ></textarea>
        <div v-else v-html="previewHTML" class="preview"></div>
      </el-form-item>

      <el-form-item>
        <el-button type="primary" @click="submit">发布</el-button>
      </el-form-item>
    </el-form>
  </el-card>
</template>

<script setup>
import { ref, onMounted, watch } from 'vue'
import { ElMessage } from 'element-plus'
import DOMPurify from 'dompurify'
import { getToken } from '../utils/storage'
import { md } from '../utils/markdown'

const form = ref({
  title: '',
  description: '',
  tags: [],
  category: '',
  content: ''
})

const formRef = ref()
const rules = {
  title: [{ required: true, message: '请输入标题', trigger: 'blur' }],
  content: [{ required: true, message: '请输入内容', trigger: 'blur' }]
}

const isPreview = ref(false)
const previewHTML = ref('')

function updatePreview() {
  previewHTML.value = DOMPurify.sanitize(
    md.render(form.value.content || '')
  )
}

// Simple debounce implementation
let updateTimer
watch(
  () => form.value.content,
  () => {
    clearTimeout(updateTimer)
    updateTimer = setTimeout(updatePreview, 300)
  },
  { immediate: true }
)

const tagOptions = ref([])
const categoryOptions = ref([])

onMounted(async () => {
  try {
    const [tagsRes, categoriesRes] = await Promise.all([
      fetch('/api/tags'),
      fetch('/api/categories')
    ])
    if (tagsRes.ok) {
      tagOptions.value = await tagsRes.json()
    }
    if (categoriesRes.ok) {
      categoryOptions.value = await categoriesRes.json()
    }
  } catch (e) {
    // ignore loading errors
  }
})

async function submit() {
  try {
    await formRef.value.validate()
  } catch {
    return
  }
  try {
    const token = getToken()
    const res = await fetch('/api/articles', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...(token ? { Authorization: `Bearer ${token}` } : {})
      },
      body: JSON.stringify(form.value)
    })
    if (res.status === 401) {
      ElMessage.error('请先登录')
      return
    }
    if (res.status === 403) {
      ElMessage.error('没有权限')
      return
    }
    if (!res.ok) {
      const msg = await res.text()
      ElMessage.error(msg || '发布失败')
      return
    }
    ElMessage.success('发布成功')
  } catch (e) {
    ElMessage.error(e.message || '发布失败')
  }
}
</script>

<style scoped>
.article-new {
  max-width: 600px;
  margin: 0 auto;
}

.editor {
  width: 100%;
  min-height: 200px;
  padding: 10px;
  border: 1px solid var(--el-border-color);
  border-radius: 4px;
}

.preview {
  border: 1px solid var(--el-border-color);
  border-radius: 4px;
  padding: 10px;
  min-height: 200px;
}
</style>

