<template>
  <el-card class="article-new">
    <el-form :model="form" label-width="80px">
      <el-form-item label="标题" required>
        <el-input v-model="form.title" placeholder="请输入标题" />
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

      <el-form-item label="描述">
        <el-input type="textarea" v-model="form.description" />
      </el-form-item>

      <el-form-item>
        <el-checkbox v-model="form.draft">草稿</el-checkbox>
      </el-form-item>

      <el-form-item label="内容" required>
        <div class="editor">
          <el-button @click="isPreview = !isPreview" class="toggle-btn">
            {{ isPreview ? '编辑' : '预览' }}
          </el-button>
          <div v-if="!isPreview" class="edit-area">
            <textarea v-model="form.content"></textarea>
          </div>
          <div v-else class="preview-area" v-html="renderedMarkdown"></div>
        </div>
      </el-form-item>

      <el-form-item>
        <el-button type="primary" @click="submit">发布</el-button>
      </el-form-item>
    </el-form>
  </el-card>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import MarkdownIt from 'markdown-it'
import DOMPurify from 'dompurify'

const form = ref({
  title: '',
  tags: [],
  category: '',
  description: '',
  draft: false,
  content: ''
})

const tagOptions = ref([])
const categoryOptions = ref([])
const isPreview = ref(false)

const md = new MarkdownIt()
const renderedMarkdown = computed(() =>
  DOMPurify.sanitize(md.render(form.value.content || ''))
)

const token = localStorage.getItem('token') || ''

async function submit() {
  if (!form.value.title || !form.value.content) {
    ElMessage.error('标题和内容为必填项')
    return
  }
  try {
    const res = await fetch('/api/articles', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: token
      },
      body: JSON.stringify(form.value)
    })
    if (!res.ok) {
      throw new Error(await res.text())
    }
    ElMessage.success('创建成功')
  } catch (e) {
    ElMessage.error(e.message || '创建失败')
  }
}

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
</script>

<style scoped>
.editor .edit-area textarea {
  width: 100%;
  min-height: 300px;
}
.preview-area {
  border: 1px solid #dcdfe6;
  padding: 10px;
  min-height: 300px;
}
.toggle-btn {
  margin-bottom: 10px;
}
</style>
