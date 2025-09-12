# Scribe

This repository contains both the backend REST API and the Vue-based frontend for managing and viewing articles.

## Backend

This project exposes a REST API for managing articles. Errors are returned in a consistent JSON format and are logged for easier debugging.

### Configuration

Runtime configuration is read from `config.toml`. Important options include:

```toml
log_level = "scribe=debug,tower_http=debug"
server_addr = "127.0.0.1:3000"
latest_articles_count = 10
enable_nested_categories = true
enable_comments = false
cache_max_capacity = 1000
cache_ttl_seconds = 60
github_redirect_url = "http://localhost:3000/api/auth/github/callback"
```

Content is loaded from the fixed `article` and `notes` directories located at the backend root, and the server watches the `article` directory for changes, automatically reloading modified files. Optional full‑text search can be enabled with `enable_full_text_search`. Comment endpoints and widgets remain disabled unless `enable_comments` is set to `true`. The `github_redirect_url` and GitHub OAuth environment variables are only required when comments are enabled.

If `base_url` is missing or empty, it defaults to `http://localhost:3000`.

### Error Codes

| Code | Description |
| --- | --- |
| `ERR_ARTICLE_NOT_FOUND` | Requested article does not exist |
| `ERR_VERSION_NOT_FOUND` | Requested article version is missing |
| `ERR_FULLTEXT_DISABLED` | Full‑text search service is not available |
| `ERR_EMPTY_SEARCH_QUERY` | Search query parameter was empty |
| `ERR_BAD_REQUEST` | Request parameters were invalid |
| `ERR_INTERNAL_SERVER` | Unexpected internal error |
| `ERR_INVALID_SESSION` | Example code for an unauthenticated session |

Responses use the shape:

```json
{"error_code": "ERR_ARTICLE_NOT_FOUND", "message": "Article with slug foo not found"}
```

### Logging

The application uses [`tracing`](https://crates.io/crates/tracing) for logging. Run the server with an appropriate `RUST_LOG` level to see messages:

```bash
RUST_LOG=error cargo run
```

Error logs can then be viewed in the console or collected by your preferred log aggregator.

### Environment Variables

The application reads the following values from the environment (or a `.env` file):

- `ADMIN_TOKEN_HASH` – SHA-256 hash of the admin token used for admin‑only routes.
- `GITHUB_CLIENT_ID` – OAuth client identifier for GitHub authentication (required when `enable_comments` is true).
- `GITHUB_CLIENT_SECRET` – OAuth client secret for GitHub authentication (required when `enable_comments` is true).
- `COOKIE_SECRET` – secret key used to sign session cookies.

### API Endpoints

The server exposes the following HTTP endpoints:

| Method | Path | Description |
| ------ | ---- | ----------- |
| GET | `/api/articles` | List articles with optional `tag`, `category`, `q`, `include_content`, `page`, and `limit` query parameters |
| GET | `/api/articles/{slug}` | Retrieve a single article by slug |
| GET | `/api/articles/{id}/versions` | List saved versions for an article |
| GET | `/api/articles/{id}/versions/{version}` | Fetch a specific version of an article |
| POST | `/api/articles/{id}/versions/{version}/restore` | Restore an article to a previous version *(admin only)* |
| GET | `/api/tags` | Retrieve all tags |
| GET | `/api/categories` | Retrieve all categories |
| GET | `/api/search` | Search articles (requires full‑text search to be enabled) |
| GET | `/api/search/popular` | List popular search queries |
| GET | `/api/auth/github/login` | Start GitHub OAuth login flow *(available only when comments are enabled)* |
| GET | `/api/auth/github/callback` | OAuth callback endpoint used after GitHub login *(available only when comments are enabled)* |

## Frontend

The frontend is a Vue 3 application.

### Project setup

```
pnpm install
```

### Compiles and hot-reloads for development

```
pnpm run serve
```

### Compiles and minifies for production

```
pnpm run build
```

### Lints and fixes files

```
pnpm run lint
```

### Customize configuration

See [Configuration Reference](https://cli.vuejs.org/config/).

