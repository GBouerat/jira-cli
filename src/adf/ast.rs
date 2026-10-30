//! Atlassian Document Format (ADF) Abstract Syntax Tree (AST) definitions.
//!
//! Models the JSON schema used by Jira Cloud REST API v3 for rich text fields
//! (descriptions, comments, worklogs, and custom fields).

use serde::{Deserialize, Serialize};

/// Root node of an Atlassian Document Format (ADF) structure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocNode {
    /// ADF schema version (typically `1`).
    pub version: u32,

    /// Node type, always `"doc"`.
    #[serde(rename = "type")]
    pub node_type: String,

    /// Child block nodes within the document.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content: Vec<Node>,
}

impl Default for DocNode {
    fn default() -> Self {
        Self {
            version: 1,
            node_type: "doc".to_string(),
            content: Vec::new(),
        }
    }
}

impl DocNode {
    /// Creates a new ADF document with the provided content nodes.
    pub fn new(content: Vec<Node>) -> Self {
        Self {
            version: 1,
            node_type: "doc".to_string(),
            content,
        }
    }

    /// Checks if the document contains no child nodes or only empty text nodes.
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
            || self.content.iter().all(|n| {
                n.content.is_empty()
                    && match &n.text {
                        Some(t) => t.trim().is_empty(),
                        None => true,
                    }
            })
    }
}

/// An individual node in the ADF document hierarchy.
///
/// Can represent block elements (e.g., `paragraph`, `heading`, `bulletList`, `codeBlock`)
/// or inline elements (e.g., `text`, `mention`, `inlineCard`, `hardBreak`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Node {
    /// Type of node (e.g., `"paragraph"`, `"text"`, `"heading"`, `"codeBlock"`, `"bulletList"`).
    #[serde(rename = "type")]
    pub node_type: String,

    /// Literal text content for text-bearing nodes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    /// Inline formatting marks applied to text (e.g., `strong`, `em`, `code`, `link`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub marks: Vec<Mark>,

    /// Nested child nodes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content: Vec<Node>,

    /// Additional node attributes (e.g., `level` for headings, `language` for code blocks).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attrs: Option<serde_json::Value>,
}

/// Text formatting mark applied to an inline text node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mark {
    /// Type of mark (e.g., `"strong"`, `"em"`, `"code"`, `"strike"`, `"underline"`, `"link"`).
    #[serde(rename = "type")]
    pub mark_type: String,

    /// Optional mark attributes (e.g., `href` for links).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attrs: Option<serde_json::Value>,
}

impl Mark {
    /// Creates a bold/strong mark.
    pub fn strong() -> Self {
        Self {
            mark_type: "strong".to_string(),
            attrs: None,
        }
    }

    /// Creates an italic/emphasis mark.
    pub fn em() -> Self {
        Self {
            mark_type: "em".to_string(),
            attrs: None,
        }
    }

    /// Creates an inline code mark.
    pub fn code() -> Self {
        Self {
            mark_type: "code".to_string(),
            attrs: None,
        }
    }

    /// Creates a strikethrough mark.
    pub fn strike() -> Self {
        Self {
            mark_type: "strike".to_string(),
            attrs: None,
        }
    }

    /// Creates a hyperlink mark.
    pub fn link(href: impl Into<String>) -> Self {
        Self {
            mark_type: "link".to_string(),
            attrs: Some(serde_json::json!({ "href": href.into() })),
        }
    }
}

impl Node {
    /// Creates a plain text node with no formatting marks.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            node_type: "text".to_string(),
            text: Some(text.into()),
            marks: Vec::new(),
            content: Vec::new(),
            attrs: None,
        }
    }

    /// Creates a text node with specific formatting marks.
    pub fn text_with_marks(text: impl Into<String>, marks: Vec<Mark>) -> Self {
        Self {
            node_type: "text".to_string(),
            text: Some(text.into()),
            marks,
            content: Vec::new(),
            attrs: None,
        }
    }

    /// Creates a paragraph block node containing the given inline nodes.
    pub fn paragraph(content: Vec<Node>) -> Self {
        Self {
            node_type: "paragraph".to_string(),
            text: None,
            marks: Vec::new(),
            content,
            attrs: None,
        }
    }

    /// Creates a heading block node (level 1-6).
    pub fn heading(level: u8, text: impl Into<String>) -> Self {
        let clamped_level = level.clamp(1, 6);
        Self {
            node_type: "heading".to_string(),
            text: None,
            marks: Vec::new(),
            content: vec![Node::text(text)],
            attrs: Some(serde_json::json!({ "level": clamped_level })),
        }
    }

    /// Creates a preformatted code block node.
    pub fn code_block(language: Option<&str>, code: impl Into<String>) -> Self {
        let attrs = language.map(|lang| serde_json::json!({ "language": lang }));
        Self {
            node_type: "codeBlock".to_string(),
            text: None,
            marks: Vec::new(),
            content: vec![Node::text(code)],
            attrs,
        }
    }

    /// Creates an unordered bullet list node.
    pub fn bullet_list(items: Vec<Node>) -> Self {
        Self {
            node_type: "bulletList".to_string(),
            text: None,
            marks: Vec::new(),
            content: items,
            attrs: None,
        }
    }

    /// Creates an ordered (numbered) list node.
    pub fn ordered_list(items: Vec<Node>) -> Self {
        Self {
            node_type: "orderedList".to_string(),
            text: None,
            marks: Vec::new(),
            content: items,
            attrs: None,
        }
    }

    /// Creates a list item node containing child blocks (typically paragraphs).
    pub fn list_item(content: Vec<Node>) -> Self {
        Self {
            node_type: "listItem".to_string(),
            text: None,
            marks: Vec::new(),
            content,
            attrs: None,
        }
    }

    /// Creates a blockquote node.
    pub fn blockquote(content: Vec<Node>) -> Self {
        Self {
            node_type: "blockquote".to_string(),
            text: None,
            marks: Vec::new(),
            content,
            attrs: None,
        }
    }

    /// Creates a horizontal rule node.
    pub fn rule() -> Self {
        Self {
            node_type: "rule".to_string(),
            text: None,
            marks: Vec::new(),
            content: Vec::new(),
            attrs: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doc_node_serialization() {
        let doc = DocNode::new(vec![
            Node::heading(1, "Task Overview"),
            Node::paragraph(vec![
                Node::text("This is "),
                Node::text_with_marks("important", vec![Mark::strong()]),
                Node::text("."),
            ]),
            Node::code_block(Some("rust"), "fn main() {}"),
        ]);

        let json = serde_json::to_string(&doc).expect("serialization failed");
        assert!(json.contains("\"type\":\"doc\""));
        assert!(json.contains("\"type\":\"heading\""));
        assert!(json.contains("\"level\":1"));
        assert!(json.contains("\"type\":\"strong\""));
        assert!(json.contains("\"language\":\"rust\""));

        let deserialized: DocNode = serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(doc, deserialized);
    }
}
