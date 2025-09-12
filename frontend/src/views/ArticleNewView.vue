<template>
  <el-card class="article-new">
    <el-form :model="form" label-width="80px">
      <el-form-item label="标题" required>
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

      <el-form-item label="内容">
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
        <el-button type="primary" @click="submit">提交</el-button>
      </el-form-item>
    </el-form>
  </el-card>
</template>

<script setup>
import { ref, onMounted, computed } from 'vue'
import { ElMessage } from 'element-plus'
import MarkdownIt from 'markdown-it'
import DOMPurify from 'dompurify'

const form = ref({
  title: '',
  description: '',
  tags: [],
  category: '',
  content: ''
})

const isPreview = ref(false)
const md = new MarkdownIt()
const previewHTML = computed(() =>
  DOMPurify.sanitize(md.render(form.value.content || ''))
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
    const res = await fetch('/api/articles', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(form.value)
    })
    if (!res.ok) {
      throw new Error(await res.text())
    }
    ElMessage.success('提交成功')
  } catch (e) {
    ElMessage.error(e.message || '提交失败')
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

