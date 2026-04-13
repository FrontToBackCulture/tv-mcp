// Markdown → TipTap JSON converter
// Used when creating/updating tasks (and other entities) so that plain-markdown
// input from MCP callers renders in the tv-client TipTap editor, which reads
// from a `description_json` column rather than plain `description`.

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Parser, Tag, TagEnd};
use serde_json::{json, Value};

struct Frame {
    kind: String,
    attrs: Option<Value>,
    content: Vec<Value>,
}

/// Convert a markdown string into a TipTap doc JSON value.
/// Supports paragraphs, headings, lists, blockquotes, code blocks, horizontal
/// rules, and inline marks (bold, italic, strike, code, link).
pub fn markdown_to_tiptap_json(md: &str) -> Value {
    let parser = Parser::new(md);
    let mut stack: Vec<Frame> = vec![Frame { kind: "doc".into(), attrs: None, content: Vec::new() }];
    let mut marks: Vec<Value> = Vec::new();

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => stack.push(Frame { kind: "paragraph".into(), attrs: None, content: Vec::new() }),
                Tag::Heading { level, .. } => {
                    let n = match level {
                        HeadingLevel::H1 => 1,
                        HeadingLevel::H2 => 2,
                        HeadingLevel::H3 => 3,
                        HeadingLevel::H4 => 4,
                        HeadingLevel::H5 => 5,
                        HeadingLevel::H6 => 6,
                    };
                    stack.push(Frame { kind: "heading".into(), attrs: Some(json!({ "level": n })), content: Vec::new() });
                }
                Tag::BlockQuote => stack.push(Frame { kind: "blockquote".into(), attrs: None, content: Vec::new() }),
                Tag::CodeBlock(kind) => {
                    let lang = match kind {
                        CodeBlockKind::Fenced(s) => s.to_string(),
                        CodeBlockKind::Indented => String::new(),
                    };
                    let attrs = if lang.is_empty() { None } else { Some(json!({ "language": lang })) };
                    stack.push(Frame { kind: "codeBlock".into(), attrs, content: Vec::new() });
                }
                Tag::List(Some(_)) => stack.push(Frame { kind: "orderedList".into(), attrs: None, content: Vec::new() }),
                Tag::List(None) => stack.push(Frame { kind: "bulletList".into(), attrs: None, content: Vec::new() }),
                Tag::Item => stack.push(Frame { kind: "listItem".into(), attrs: None, content: Vec::new() }),
                Tag::Emphasis => marks.push(json!({ "type": "italic" })),
                Tag::Strong => marks.push(json!({ "type": "bold" })),
                Tag::Strikethrough => marks.push(json!({ "type": "strike" })),
                Tag::Link { dest_url, .. } => {
                    marks.push(json!({ "type": "link", "attrs": { "href": dest_url.to_string() } }));
                }
                // Unsupported block/inline tags: push a passthrough frame so End matches.
                _ => stack.push(Frame { kind: "_skip_".into(), attrs: None, content: Vec::new() }),
            },
            Event::End(end) => match end {
                TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough | TagEnd::Link => {
                    marks.pop();
                }
                _ => {
                    let frame = stack.pop().expect("stack underflow");
                    if frame.kind == "_skip_" {
                        // If the skipped block collected any inline text, surface it as a paragraph
                        // so content isn't silently dropped.
                        if !frame.content.is_empty() {
                            let node = json!({ "type": "paragraph", "content": frame.content });
                            stack.last_mut().unwrap().content.push(node);
                        }
                        continue;
                    }
                    let content = if frame.kind == "listItem" {
                        // TipTap listItem requires block children (paragraph/list). Wrap bare inline.
                        if frame.content.is_empty() {
                            vec![json!({ "type": "paragraph" })]
                        } else if frame.content.iter().any(|v| {
                            matches!(v.get("type").and_then(|t| t.as_str()), Some("text") | Some("hardBreak"))
                        }) {
                            vec![json!({ "type": "paragraph", "content": frame.content })]
                        } else {
                            frame.content
                        }
                    } else {
                        frame.content
                    };
                    let mut node = json!({ "type": frame.kind });
                    if let Some(a) = frame.attrs {
                        node["attrs"] = a;
                    }
                    if !content.is_empty() {
                        node["content"] = Value::Array(content);
                    }
                    stack.last_mut().unwrap().content.push(node);
                }
            },
            Event::Text(text) => {
                let mut node = json!({ "type": "text", "text": text.to_string() });
                if !marks.is_empty() {
                    node["marks"] = Value::Array(marks.clone());
                }
                stack.last_mut().unwrap().content.push(node);
            }
            Event::Code(text) => {
                let mut m = marks.clone();
                m.push(json!({ "type": "code" }));
                let node = json!({ "type": "text", "text": text.to_string(), "marks": m });
                stack.last_mut().unwrap().content.push(node);
            }
            Event::SoftBreak => {
                stack.last_mut().unwrap().content.push(json!({ "type": "text", "text": " " }));
            }
            Event::HardBreak => {
                stack.last_mut().unwrap().content.push(json!({ "type": "hardBreak" }));
            }
            Event::Rule => {
                stack.last_mut().unwrap().content.push(json!({ "type": "horizontalRule" }));
            }
            Event::Html(_) | Event::InlineHtml(_) | Event::FootnoteReference(_) | Event::TaskListMarker(_) => {}
        }
    }

    let root = stack.pop().expect("stack empty");
    let content = if root.content.is_empty() {
        vec![json!({ "type": "paragraph" })]
    } else {
        root.content
    };
    json!({ "type": "doc", "content": content })
}
