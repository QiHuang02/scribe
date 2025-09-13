import { createRouter, createWebHistory } from 'vue-router'
import HomeView from '../views/HomeView.vue'
import ArticleView from '../views/ArticleView.vue'
import ArticleVersionView from '../views/ArticleVersionView.vue'
import CategoriesView from '../views/CategoriesView.vue'
import TagsView from '../views/TagsView.vue'
import SearchView from '../views/SearchView.vue'
import NotesView from '../views/NotesView.vue'
import NoteView from '../views/NoteView.vue'
import LoginView from '../views/LoginView.vue'
import AuthCallbackView from '../views/AuthCallbackView.vue'
import AuthGuardView from '../views/AuthGuard.vue'
import useAuth from '../composables/useAuth'
import { ElMessage } from 'element-plus'

const routes = [
  {
    path: '/',
    name: 'home',
    component: HomeView
  },
  {
    path: '/categories',
    name: 'categories',
    component: CategoriesView
  },
  {
    path: '/tags',
    name: 'tags',
    component: TagsView
  },
  {
    path: '/notes',
    name: 'notes',
    component: NotesView
  },
  {
    path: '/notes/:slug(.*)',
    name: 'note',
    component: NoteView
  },
  {
    path: '/about',
    name: 'about',
    // route level code-splitting
    // this generates a separate chunk (about.[hash].js) for this route
    // which is lazy-loaded when the route is visited.
    component: () => import(/* webpackChunkName: "about" */ '../views/AboutView.vue')
  },
  {
    path: '/articles/new',
    component: () => import('../views/ArticleNewView.vue'),
    meta: { requiresAuthor: true }
  },
  {
    path: '/articles/:slug',
    name: 'article',
    component: ArticleView
  },
  {
    path: '/articles/:slug/versions/:version',
    name: 'article-version',
    component: ArticleVersionView
  },
  {
    path: '/search',
    name: 'search',
    component: SearchView
  },
  {
    path: '/login',
    name: 'login',
    component: LoginView
  },
  {
    path: '/auth/callback',
    name: 'auth-callback',
    component: AuthCallbackView
  },
  {
    path: '/forbidden',
    name: 'forbidden',
    component: AuthGuardView
  }
]

const router = createRouter({
  history: createWebHistory(process.env.BASE_URL),
  routes
})

const { isAuthenticated, isAuthor } = useAuth()

router.beforeEach((to, from, next) => {
  if (to.meta.requiresAuthor && !isAuthor.value) {
    ElMessage.error('Insufficient permissions')
    next({ name: 'forbidden' })
    return
  }
  if (to.meta.requiresAuth && !isAuthenticated.value) {
    ElMessage.error('Insufficient permissions')
    next({ name: 'forbidden' })
    return
  }
  next()
})

export default router
