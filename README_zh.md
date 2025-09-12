# Scribe

该仓库包含用于管理和查看文章的后端 REST API 和基于 Vue 的前端。

## 后端

该项目提供用于管理文章的 REST API。错误以一致的 JSON 格式返回，并会记录日志以便调试。

### 配置

运行时配置从 `config.toml` 读取。重要选项包括：

```toml
article_dir = "article"
log_level = "scribe=debug,tower_http=debug"
server_addr = "127.0.0.1:3000"
latest_articles_count = 10
enable_nested_categories = true
enable_comments = false
cache_max_capacity = 1000
cache_ttl_seconds = 60
github_redirect_url = "http://localhost:3000/api/auth/github/callback"
```

服务器会监视 `article_dir` 的变化并自动重新加载被修改的文件。可选的全文搜索可以通过 `enable_full_text_search` 启用。评论端点和小部件默认关闭，除非将 `enable_comments` 设置为 `true`。只有在启用评论功能时才需要 `github_redirect_url` 和相关的 GitHub OAuth 环境变量。

### 错误码

| 代码 | 描述 |
| --- | --- |
| `ERR_ARTICLE_NOT_FOUND` | 请求的文章不存在 |
| `ERR_VERSION_NOT_FOUND` | 请求的文章版本缺失 |
| `ERR_FULLTEXT_DISABLED` | 全文搜索服务不可用 |
| `ERR_EMPTY_SEARCH_QUERY` | 搜索查询参数为空 |
| `ERR_BAD_REQUEST` | 请求参数无效 |
| `ERR_INTERNAL_SERVER` | 未预期的内部错误 |
| `ERR_INVALID_SESSION` | 未认证会话示例 |

响应使用如下格式：

```json
{"error_code": "ERR_ARTICLE_NOT_FOUND", "message": "Article with slug foo not found"}
```

### 日志

应用使用 [`tracing`](https://crates.io/crates/tracing) 进行日志记录。运行服务器时配置合适的 `RUST_LOG` 级别以查看消息：

```bash
RUST_LOG=error cargo run
```

然后可以在控制台查看错误日志或将其收集到你的日志聚合器中。

### 环境变量

应用从环境（或 `.env` 文件）读取以下值：

- `ADMIN_TOKEN_HASH` – 管理员路由所用 token 的 SHA-256 哈希。
- `GITHUB_CLIENT_ID` – GitHub 认证用的 OAuth client ID（仅当 `enable_comments` 为 `true` 时需要）。
- `GITHUB_CLIENT_SECRET` – GitHub 认证用的 OAuth client secret（仅当 `enable_comments` 为 `true` 时需要）。
- `COOKIE_SECRET` – 用于签名会话 cookie 的密钥。

### API 端点

服务器暴露以下 HTTP 端点：

| 方法 | 路径 | 描述 |
| ---- | ---- | ---- |
| GET | `/api/articles` | 列出文章，可选查询参数：`tag`、`category`、`q`、`include_content`、`page`、`limit` |
| GET | `/api/articles/{slug}` | 通过 slug 获取文章 |
| GET | `/api/articles/{id}/versions` | 列出文章保存的版本 |
| GET | `/api/articles/{id}/versions/{version}` | 获取文章的指定版本 |
| POST | `/api/articles/{id}/versions/{version}/restore` | 将文章恢复到指定版本（仅管理员） |
| GET | `/api/tags` | 获取所有标签 |
| GET | `/api/categories` | 获取所有分类 |
| GET | `/api/search` | 搜索文章（需要启用全文搜索） |
| GET | `/api/search/popular` | 列出热门搜索 |
| GET | `/api/auth/github/login` | 启动 GitHub OAuth 登录流程（仅在启用评论功能时可用） |
| GET | `/api/auth/github/callback` | GitHub 登录完成后的回调端点（仅在启用评论功能时可用） |

## 前端

前端是一个基于 Vue 3 的应用。

### 项目安装

```
pnpm install
```

### 开发环境热重载

```
pnpm run serve
```

### 生产环境构建

```
pnpm run build
```

### Lint 并修复文件

```
pnpm run lint
```

### 自定义配置

参见 [Configuration Reference](https://cli.vuejs.org/config/).

