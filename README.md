# Scribe

This project exposes a REST API for managing articles. Errors are returned in a
consistent JSON format and are logged for easier debugging.

## Error Codes

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

## Logging

The application uses [`tracing`](https://crates.io/crates/tracing) for logging.
Run the server with an appropriate `RUST_LOG` level to see messages:

```bash
RUST_LOG=error cargo run
```

Error logs can then be viewed in the console or collected by your preferred log
aggregator.

## Environment Variables

The application reads the following values from the environment (or a `.env` file):

- `ADMIN_TOKEN_HASH` – SHA-256 hash of the admin token used for admin‑only routes.
- `GITHUB_CLIENT_ID` – OAuth client identifier for GitHub authentication.
- `GITHUB_CLIENT_SECRET` – OAuth client secret for GitHub authentication.

