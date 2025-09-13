use ammonia::Builder;
use pulldown_cmark::{Options, Parser, html};
use std::fs;
use std::path::Path;

/// Convert Markdown into sanitized HTML and persist to the provided path.
///
/// This function first renders the provided Markdown to HTML, then uses
/// [`ammonia`] to strip any potentially harmful tags or attributes before
/// writing the result to disk.
pub fn save_comment<P: AsRef<Path>>(raw_markdown: &str, path: P) -> std::io::Result<()> {
    let sanitized = sanitize_content(raw_markdown);
    fs::write(path, sanitized)
}

/// Convert Markdown text to sanitized HTML.
fn sanitize_content(raw_markdown: &str) -> String {
    let parser = Parser::new_ext(raw_markdown, Options::all());
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    Builder::default().clean(&html_output).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn strips_script_tags() {
        let input = "Hello<script>alert('xss');</script>";
        let sanitized = sanitize_content(input);
        assert!(!sanitized.contains("<script"));
    }

    #[test]
    fn converts_markdown_and_sanitizes() {
        let input = "**bold**";
        let sanitized = sanitize_content(input);
        assert!(sanitized.contains("<strong>bold</strong>"));
    }

    #[test]
    fn persists_sanitized_content() {
        let input = "Hello<script>alert('xss');</script>";
        let mut tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        save_comment(input, &path).unwrap();
        tmp.flush().unwrap();
        let stored = std::fs::read_to_string(path).unwrap();
        assert!(!stored.contains("<script"));
    }
}
