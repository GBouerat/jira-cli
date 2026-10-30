//! Converts Atlassian Document Format (ADF) AST into terminal-friendly formatted text.
//!
//! Handles formatting marks (bold, italic, code, hyperlinks), block structures
//! (headings, bullet/ordered lists, blockquotes, code blocks), and gracefully falls
//! back on unknown node types.

use crate::adf::ast::{DocNode, Node};
use colored::Colorize;

/// Renders an ADF [`DocNode`] into a string suitable for terminal display.
///
/// # Arguments
/// * `doc` - The root ADF document node.
/// * `colored` - Whether to apply ANSI color codes or plain text Markdown markers.
pub fn adf_to_terminal(doc: &DocNode, colored: bool) -> String {
    let mut output = String::new();
    for block in &doc.content {
        render_block(block, &mut output, 0, colored);
    }
    output.trim().to_string()
}

fn render_block(node: &Node, out: &mut String, indent_level: usize, colored: bool) {
    let indent = "  ".repeat(indent_level);

    match node.node_type.as_str() {
        "paragraph" => {
            out.push_str(&indent);
            for inline in &node.content {
                render_inline(inline, out, colored);
            }
            out.push_str("\n\n");
        }

        "heading" => {
            let level = node
                .attrs
                .as_ref()
                .and_then(|a| a.get("level"))
                .and_then(|l| l.as_u64())
                .unwrap_or(1) as usize;

            let mut heading_text = String::new();
            for inline in &node.content {
                render_inline(inline, &mut heading_text, false);
            }

            out.push_str(&indent);
            if colored {
                match level {
                    1 => out.push_str(&format!("{}\n\n", heading_text.bold().cyan())),
                    2 => out.push_str(&format!("{}\n\n", heading_text.bold().yellow())),
                    _ => out.push_str(&format!("{}\n\n", heading_text.bold())),
                }
            } else {
                let hashes = "#".repeat(level);
                out.push_str(&format!("{} {}\n\n", hashes, heading_text));
            }
        }

        "codeBlock" => {
            let lang = node
                .attrs
                .as_ref()
                .and_then(|a| a.get("language"))
                .and_then(|l| l.as_str())
                .unwrap_or("");

            let mut code = String::new();
            for inline in &node.content {
                if let Some(t) = &inline.text {
                    code.push_str(t);
                }
            }

            out.push_str(&indent);
            if colored {
                out.push_str(&format!("```{}\n", lang).dimmed().to_string());
                for line in code.lines() {
                    out.push_str(&format!("{}{}\n", indent, line.green()));
                }
                out.push_str(&format!("{}```\n\n", indent).dimmed().to_string());
            } else {
                out.push_str(&format!("```{}\n", lang));
                for line in code.lines() {
                    out.push_str(&format!("{}{}\n", indent, line));
                }
                out.push_str(&format!("{}```\n\n", indent));
            }
        }

        "bulletList" => {
            for item in &node.content {
                if item.node_type == "listItem" {
                    render_list_item(item, out, indent_level, "• ", colored);
                } else {
                    render_block(item, out, indent_level + 1, colored);
                }
            }
            out.push('\n');
        }

        "orderedList" => {
            for (idx, item) in node.content.iter().enumerate() {
                if item.node_type == "listItem" {
                    let bullet = format!("{}. ", idx + 1);
                    render_list_item(item, out, indent_level, &bullet, colored);
                } else {
                    render_block(item, out, indent_level + 1, colored);
                }
            }
            out.push('\n');
        }

        "blockquote" => {
            let mut quote_body = String::new();
            for child in &node.content {
                render_block(child, &mut quote_body, 0, colored);
            }
            for line in quote_body.trim_end().lines() {
                out.push_str(&indent);
                if colored {
                    out.push_str(&format!("{} {}\n", "│".dimmed(), line));
                } else {
                    out.push_str(&format!("> {}\n", line));
                }
            }
            out.push('\n');
        }

        "rule" => {
            out.push_str(&indent);
            if colored {
                out.push_str(&"────────────────────────────────────────\n\n".dimmed().to_string());
            } else {
                out.push_str("----------------------------------------\n\n");
            }
        }

        "panel" => {
            let panel_type = node
                .attrs
                .as_ref()
                .and_then(|a| a.get("panelType"))
                .and_then(|p| p.as_str())
                .unwrap_or("info");

            let header = match panel_type {
                "warning" => "⚠ WARNING",
                "error" => "✖ ERROR",
                "success" => "✔ SUCCESS",
                "note" => "ℹ NOTE",
                _ => "ℹ INFO",
            };

            out.push_str(&indent);
            if colored {
                out.push_str(&format!("[{}]\n", header).bold().blue().to_string());
            } else {
                out.push_str(&format!("[{}]\n", header));
            }

            for child in &node.content {
                render_block(child, out, indent_level + 1, colored);
            }
        }

        // Fallback for unknown block nodes: render children or text
        _ => {
            if let Some(t) = &node.text {
                out.push_str(&indent);
                out.push_str(t);
                out.push_str("\n\n");
            }
            for child in &node.content {
                render_block(child, out, indent_level, colored);
            }
        }
    }
}

fn render_list_item(item: &Node, out: &mut String, indent_level: usize, bullet: &str, colored: bool) {
    let indent = "  ".repeat(indent_level);
    out.push_str(&indent);
    if colored {
        out.push_str(&bullet.cyan().to_string());
    } else {
        out.push_str(bullet);
    }

    let mut first = true;
    for child in &item.content {
        if child.node_type == "paragraph" {
            if !first {
                out.push_str(&indent);
                out.push_str("  ");
            }
            for inline in &child.content {
                render_inline(inline, out, colored);
            }
            out.push('\n');
            first = false;
        } else {
            render_block(child, out, indent_level + 1, colored);
        }
    }
}

fn render_inline(node: &Node, out: &mut String, colored: bool) {
    match node.node_type.as_str() {
        "text" => {
            let raw_text = node.text.as_deref().unwrap_or("");
            if raw_text.is_empty() {
                return;
            }

            if !colored {
                let mut formatted = raw_text.to_string();
                for mark in &node.marks {
                    match mark.mark_type.as_str() {
                        "strong" => formatted = format!("**{}**", formatted),
                        "em" => formatted = format!("*{}*", formatted),
                        "code" => formatted = format!("`{}`", formatted),
                        "strike" => formatted = format!("~~{}~~", formatted),
                        "link" => {
                            if let Some(href) = mark.attrs.as_ref().and_then(|a| a.get("href")).and_then(|h| h.as_str()) {
                                formatted = format!("[{}]({})", formatted, href);
                            }
                        }
                        _ => {}
                    }
                }
                out.push_str(&formatted);
            } else {
                let mut styled = raw_text.normal();
                let mut link_href: Option<&str> = None;

                for mark in &node.marks {
                    match mark.mark_type.as_str() {
                        "strong" => styled = styled.bold(),
                        "em" => styled = styled.italic(),
                        "code" => styled = styled.yellow(),
                        "strike" => styled = styled.strikethrough(),
                        "underline" => styled = styled.underline(),
                        "link" => {
                            link_href = mark
                                .attrs
                                .as_ref()
                                .and_then(|a| a.get("href"))
                                .and_then(|h| h.as_str());
                        }
                        _ => {}
                    }
                }

                if let Some(href) = link_href {
                    out.push_str(&format!("{} ({})", styled.blue().underline(), href.dimmed()));
                } else {
                    out.push_str(&styled.to_string());
                }
            }
        }

        "hardBreak" => {
            out.push('\n');
        }

        "mention" => {
            let mention_text = node
                .attrs
                .as_ref()
                .and_then(|a| a.get("text"))
                .and_then(|t| t.as_str())
                .unwrap_or("@user");

            if colored {
                out.push_str(&mention_text.magenta().bold().to_string());
            } else {
                out.push_str(mention_text);
            }
        }

        _ => {
            if let Some(t) = &node.text {
                out.push_str(t);
            }
            for child in &node.content {
                render_inline(child, out, colored);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adf::ast::{DocNode, Mark, Node};

    #[test]
    fn test_adf_to_terminal_plain() {
        let doc = DocNode::new(vec![
            Node::heading(1, "Overview"),
            Node::paragraph(vec![
                Node::text("Here is "),
                Node::text_with_marks("bold", vec![Mark::strong()]),
                Node::text(" text."),
            ]),
        ]);

        let plain = adf_to_terminal(&doc, false);
        assert!(plain.contains("# Overview"));
        assert!(plain.contains("Here is **bold** text."));
    }

    #[test]
    fn test_adf_to_terminal_bullet_list() {
        let doc = DocNode::new(vec![Node::bullet_list(vec![
            Node::list_item(vec![Node::paragraph(vec![Node::text("Item A")])]),
            Node::list_item(vec![Node::paragraph(vec![Node::text("Item B")])]),
        ])]);

        let plain = adf_to_terminal(&doc, false);
        assert!(plain.contains("• Item A"));
        assert!(plain.contains("• Item B"));
    }
}
