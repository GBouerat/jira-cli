//! Converts Markdown or plain text into Atlassian Document Format (ADF) AST.
//!
//! Enables users to write intuitive Markdown in CLI prompts and flags,
//! automatically compiling it into valid Jira Cloud REST API v3 ADF payloads.

use crate::adf::ast::{DocNode, Mark, Node};

/// Parses a Markdown-like string into an ADF [`DocNode`].
///
/// Supports headings (`#`), bullet lists (`- `, `* `), numbered lists (`1. `),
/// blockquotes (`> `), code fences (````), horizontal rules (`---`), and
/// inline formatting (`**bold**`, `*italic*`, `` `code` ``, `[text](url)`).
pub fn text_to_adf(input: &str) -> DocNode {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return DocNode::new(vec![Node::paragraph(Vec::new())]);
    }

    let mut blocks: Vec<Node> = Vec::new();
    let lines: Vec<&str> = trimmed.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed_line = line.trim();

        // 1. Code block fence (```)
        if trimmed_line.starts_with("```") {
            let lang = trimmed_line.trim_start_matches('`').trim();
            let language = if lang.is_empty() { None } else { Some(lang) };
            let mut code_lines = Vec::new();
            i += 1;
            while i < lines.len() && !lines[i].trim().starts_with("```") {
                code_lines.push(lines[i]);
                i += 1;
            }
            if i < lines.len() {
                i += 1; // skip closing fence
            }
            let code_content = code_lines.join("\n");
            blocks.push(Node::code_block(language, code_content));
            continue;
        }

        // 2. Horizontal rule
        if trimmed_line == "---" || trimmed_line == "***" || trimmed_line == "___" {
            blocks.push(Node::rule());
            i += 1;
            continue;
        }

        // 3. Headings (# Title)
        if trimmed_line.starts_with('#') {
            let hash_count = trimmed_line.chars().take_while(|c| *c == '#').count();
            if hash_count <= 6 && trimmed_line[hash_count..].starts_with(' ') {
                let heading_text = trimmed_line[hash_count..].trim();
                blocks.push(Node::heading(hash_count as u8, heading_text));
                i += 1;
                continue;
            }
        }

        // 4. Blockquotes (> Quote)
        if trimmed_line.starts_with("> ") || trimmed_line == ">" {
            let mut quote_lines = Vec::new();
            while i < lines.len() && (lines[i].trim().starts_with("> ") || lines[i].trim() == ">") {
                let q_line = lines[i].trim();
                let stripped = if q_line == ">" { "" } else { &q_line[2..] };
                quote_lines.push(stripped);
                i += 1;
            }
            let quote_text = quote_lines.join("\n");
            let inner_paragraphs = vec![Node::paragraph(parse_inline(&quote_text))];
            blocks.push(Node::blockquote(inner_paragraphs));
            continue;
        }

        // 5. Bullet list (- item or * item)
        if is_bullet_item(trimmed_line) {
            let mut items = Vec::new();
            while i < lines.len() && is_bullet_item(lines[i].trim()) {
                let item_text = extract_bullet_text(lines[i].trim());
                let item_node = Node::list_item(vec![Node::paragraph(parse_inline(item_text))]);
                items.push(item_node);
                i += 1;
            }
            blocks.push(Node::bullet_list(items));
            continue;
        }

        // 6. Numbered list (1. item)
        if is_ordered_item(trimmed_line) {
            let mut items = Vec::new();
            while i < lines.len() && is_ordered_item(lines[i].trim()) {
                let item_text = extract_ordered_text(lines[i].trim());
                let item_node = Node::list_item(vec![Node::paragraph(parse_inline(item_text))]);
                items.push(item_node);
                i += 1;
            }
            blocks.push(Node::ordered_list(items));
            continue;
        }

        // 7. Regular paragraph (collect non-empty lines until blank line or special block)
        if !trimmed_line.is_empty() {
            let mut para_lines = Vec::new();
            while i < lines.len() {
                let curr = lines[i].trim();
                if curr.is_empty()
                    || curr.starts_with("```")
                    || curr.starts_with('#')
                    || curr.starts_with("> ")
                    || is_bullet_item(curr)
                    || is_ordered_item(curr)
                    || curr == "---"
                {
                    break;
                }
                para_lines.push(lines[i]);
                i += 1;
            }
            let para_text = para_lines.join(" ");
            blocks.push(Node::paragraph(parse_inline(&para_text)));
            continue;
        }

        i += 1;
    }

    if blocks.is_empty() {
        blocks.push(Node::paragraph(Vec::new()));
    }

    DocNode::new(blocks)
}

fn is_bullet_item(s: &str) -> bool {
    s.starts_with("- ") || s.starts_with("* ")
}

fn extract_bullet_text(s: &str) -> &str {
    s[2..].trim()
}

fn is_ordered_item(s: &str) -> bool {
    let mut chars = s.chars();
    let digits = chars.by_ref().take_while(|c| c.is_ascii_digit()).count();
    digits > 0 && s[digits..].starts_with(". ")
}

fn extract_ordered_text(s: &str) -> &str {
    let dot_idx = s.find(". ").unwrap_or(0);
    s[dot_idx + 2..].trim()
}

/// Parses inline formatting marks (`**bold**`, `__bold__`, `*italic*`, `_italic_`, `~~strike~~`, `` `code` ``, `[text](url)`).
pub fn parse_inline(text: &str) -> Vec<Node> {
    if text.is_empty() {
        return Vec::new();
    }

    let mut nodes: Vec<Node> = Vec::new();
    let mut chars = text.chars().peekable();
    let mut current_buf = String::new();

    while let Some(c) = chars.next() {
        // Strikethrough: ~~text~~
        if c == '~' && chars.peek() == Some(&'~') {
            chars.next(); // consume second '~'
            let mut strike_buf = String::new();
            let mut closed = false;
            while let Some(next_c) = chars.next() {
                if next_c == '~' && chars.peek() == Some(&'~') {
                    chars.next();
                    closed = true;
                    break;
                }
                strike_buf.push(next_c);
            }
            if closed {
                if !current_buf.is_empty() {
                    nodes.push(Node::text(std::mem::take(&mut current_buf)));
                }
                nodes.push(Node::text_with_marks(strike_buf, vec![Mark::strike()]));
            } else {
                current_buf.push_str("~~");
                current_buf.push_str(&strike_buf);
            }
            continue;
        }

        // Inline code: `code`
        if c == '`' {
            let mut code_buf = String::new();
            let mut closed = false;
            for next_c in chars.by_ref() {
                if next_c == '`' {
                    closed = true;
                    break;
                }
                code_buf.push(next_c);
            }
            if closed {
                if !current_buf.is_empty() {
                    nodes.push(Node::text(std::mem::take(&mut current_buf)));
                }
                nodes.push(Node::text_with_marks(code_buf, vec![Mark::code()]));
            } else {
                current_buf.push('`');
                current_buf.push_str(&code_buf);
            }
            continue;
        }

        // Bold (**text** or __text__) or Italic (*text* or _text_)
        if c == '*' || c == '_' {
            let target_char = c;
            if chars.peek() == Some(&target_char) {
                chars.next(); // consume second delimiter
                let mut bold_buf = String::new();
                let mut closed = false;
                while let Some(next_c) = chars.next() {
                    if next_c == target_char && chars.peek() == Some(&target_char) {
                        chars.next();
                        closed = true;
                        break;
                    }
                    bold_buf.push(next_c);
                }
                if closed {
                    if !current_buf.is_empty() {
                        nodes.push(Node::text(std::mem::take(&mut current_buf)));
                    }
                    nodes.push(Node::text_with_marks(bold_buf, vec![Mark::strong()]));
                } else {
                    current_buf.push(target_char);
                    current_buf.push(target_char);
                    current_buf.push_str(&bold_buf);
                }
            } else {
                let mut italic_buf = String::new();
                let mut closed = false;
                for next_c in chars.by_ref() {
                    if next_c == target_char {
                        closed = true;
                        break;
                    }
                    italic_buf.push(next_c);
                }
                if closed {
                    if !current_buf.is_empty() {
                        nodes.push(Node::text(std::mem::take(&mut current_buf)));
                    }
                    nodes.push(Node::text_with_marks(italic_buf, vec![Mark::em()]));
                } else {
                    current_buf.push(target_char);
                    current_buf.push_str(&italic_buf);
                }
            }
            continue;
        }

        // Hyperlinks: [label](url)
        if c == '[' {
            let mut label_buf = String::new();
            let mut closed_bracket = false;
            for next_c in chars.by_ref() {
                if next_c == ']' {
                    closed_bracket = true;
                    break;
                }
                label_buf.push(next_c);
            }

            if closed_bracket && chars.peek() == Some(&'(') {
                chars.next(); // consume '('
                let mut url_buf = String::new();
                let mut closed_paren = false;
                for next_c in chars.by_ref() {
                    if next_c == ')' {
                        closed_paren = true;
                        break;
                    }
                    url_buf.push(next_c);
                }

                if closed_paren {
                    if !current_buf.is_empty() {
                        nodes.push(Node::text(std::mem::take(&mut current_buf)));
                    }
                    nodes.push(Node::text_with_marks(label_buf, vec![Mark::link(url_buf)]));
                    continue;
                } else {
                    current_buf.push('[');
                    current_buf.push_str(&label_buf);
                    current_buf.push_str("](");
                    current_buf.push_str(&url_buf);
                    continue;
                }
            } else {
                current_buf.push('[');
                current_buf.push_str(&label_buf);
                if closed_bracket {
                    current_buf.push(']');
                }
                continue;
            }
        }

        current_buf.push(c);
    }

    if !current_buf.is_empty() {
        nodes.push(Node::text(current_buf));
    }

    nodes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_to_adf_heading_and_paragraph() {
        let input = "# Main Title\n\nThis is a **bold** paragraph with `code`.\n\n## Subtitle";
        let doc = text_to_adf(input);

        assert_eq!(doc.content.len(), 3);
        assert_eq!(doc.content[0].node_type, "heading");
        assert_eq!(doc.content[1].node_type, "paragraph");
        assert_eq!(doc.content[2].node_type, "heading");

        // Verify inline formatting in paragraph
        let p_content = &doc.content[1].content;
        assert_eq!(p_content[0].text.as_deref(), Some("This is a "));
        assert_eq!(p_content[1].text.as_deref(), Some("bold"));
        assert_eq!(p_content[1].marks[0].mark_type, "strong");
        assert_eq!(p_content[3].text.as_deref(), Some("code"));
        assert_eq!(p_content[3].marks[0].mark_type, "code");
    }

    #[test]
    fn test_text_to_adf_code_block() {
        let input = "```rust\nfn hello() {\n    println!(\"hello\");\n}\n```";
        let doc = text_to_adf(input);

        assert_eq!(doc.content.len(), 1);
        assert_eq!(doc.content[0].node_type, "codeBlock");
        assert_eq!(
            doc.content[0].attrs,
            Some(serde_json::json!({ "language": "rust" }))
        );
    }

    #[test]
    fn test_text_to_adf_bullet_list() {
        let input = "- First item\n- Second item\n- Third item";
        let doc = text_to_adf(input);

        assert_eq!(doc.content.len(), 1);
        assert_eq!(doc.content[0].node_type, "bulletList");
        assert_eq!(doc.content[0].content.len(), 3);
        assert_eq!(doc.content[0].content[0].node_type, "listItem");
    }

    #[test]
    fn test_text_to_adf_links() {
        let input = "Visit [Atlassian](https://atlassian.com) for details.";
        let doc = text_to_adf(input);

        let p_content = &doc.content[0].content;
        assert_eq!(p_content[1].text.as_deref(), Some("Atlassian"));
        assert_eq!(p_content[1].marks[0].mark_type, "link");
    }

    #[test]
    fn test_text_to_adf_strikethrough_and_underscores() {
        let input = "This is ~~deleted~~ and __important__ and _emphasized_.";
        let doc = text_to_adf(input);

        let p = &doc.content[0].content;
        assert_eq!(p[1].text.as_deref(), Some("deleted"));
        assert_eq!(p[1].marks[0].mark_type, "strike");

        assert_eq!(p[3].text.as_deref(), Some("important"));
        assert_eq!(p[3].marks[0].mark_type, "strong");

        assert_eq!(p[5].text.as_deref(), Some("emphasized"));
        assert_eq!(p[5].marks[0].mark_type, "em");
    }

    #[test]
    fn test_text_to_adf_empty_input_schema_compliance() {
        let doc = text_to_adf("");
        assert_eq!(doc.content.len(), 1);
        assert_eq!(doc.content[0].node_type, "paragraph");
        assert!(
            doc.content[0].content.is_empty(),
            "Empty paragraph must have no child text nodes"
        );
    }
}
