CREATE TABLE comments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    article_slug TEXT NOT NULL,
    author_github_id INTEGER NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_comments_article_slug ON comments(article_slug);
CREATE INDEX idx_comments_author_github_id ON comments(author_github_id);
