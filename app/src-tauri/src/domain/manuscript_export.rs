//! Human-readable export is derived from validated manuscripts, never canonical storage.
use crate::domain::{story::document_text, structure::ChapterId};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportChapter {
    pub id: ChapterId,
    pub title: String,
    pub workspace_state: String,
    pub word_count: usize,
}

#[derive(Debug, Clone)]
pub struct ManuscriptSelection {
    pub global_revision: i64,
    pub working_name: String,
    pub chapters: Vec<ExportChapter>,
    pub markdown: String,
    pub word_count: usize,
    pub format_notes: Vec<String>,
}

fn children(node: &Value) -> &[Value] {
    node.get("content")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}
fn kind(node: &Value) -> &str {
    node["type"].as_str().unwrap_or("")
}
pub fn escape_markdown(text: &str) -> String {
    let mut result = String::new();
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut line_start = true;
    for c in normalized.chars() {
        if line_start && matches!(c, ' ' | '\t') {
            result.push_str(if c == ' ' { "&#32;" } else { "&#9;" });
            continue;
        }
        line_start = c == '\n';
        match c {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '\n' => result.push_str("  \n"),
            '\r' => {}
            c if c.is_ascii_punctuation() => {
                result.push('\\');
                result.push(c);
            }
            c => result.push(c),
        }
    }
    result
}
fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
fn marks(node: &Value) -> Vec<&str> {
    let marks = node.get("marks").and_then(Value::as_array);
    ["bold", "italic", "strike", "underline", "code"]
        .into_iter()
        .filter(|kind| {
            marks.is_some_and(|marks| marks.iter().any(|m| m["type"].as_str() == Some(kind)))
        })
        .collect()
}
fn mark_tag(mark: &str) -> &str {
    match mark {
        "bold" => "strong",
        "italic" => "em",
        "strike" => "del",
        "underline" => "u",
        "code" => "code",
        _ => unreachable!(),
    }
}
fn inline(node: &Value, html: bool, notes: &mut bool) -> String {
    let mut output = String::new();
    for child in children(node) {
        let marks = marks(child);
        if !marks.is_empty() {
            *notes = true;
        }
        // Generated tags contain no author-controlled attributes. Inline HTML
        // avoids ambiguous adjoining Markdown delimiters and preserves marks
        // around punctuation, Unicode and leading/trailing whitespace exactly.
        for mark in &marks {
            output.push_str(&format!("<{}>", mark_tag(mark)));
        }
        if kind(child) == "hardBreak" {
            output.push_str(if html { "<br>\n" } else { "  \n" });
        } else {
            let text = child["text"].as_str().unwrap_or("");
            if marks.contains(&"code") {
                // Entities keep code literal, including Markdown punctuation,
                // whitespace and embedded newlines, inside the generated tag.
                for c in text.chars() {
                    if c.is_ascii_punctuation() || c.is_ascii_whitespace() {
                        output.push_str(&format!("&#{};", c as u32));
                    } else {
                        output.push(c);
                    }
                }
            } else if html {
                output.push_str(&escape_html(text).replace('\n', "<br>\n"));
            } else {
                output.push_str(&escape_markdown(text));
            }
        }
        for mark in marks.iter().rev() {
            output.push_str(&format!("</{}>", mark_tag(mark)));
        }
    }
    output
}
fn html_block(node: &Value, notes: &mut bool) -> String {
    let tag = match kind(node) {
        "paragraph" => "p".into(),
        "heading" => format!("h{}", node["attrs"]["level"].as_i64().unwrap()),
        "blockquote" => "blockquote".into(),
        "bulletList" => "ul".into(),
        "orderedList" => "ol".into(),
        "listItem" => "li".into(),
        "horizontalRule" => return "<hr>\n".into(),
        _ => unreachable!(),
    };
    let attrs = if kind(node) == "orderedList" {
        let start = node
            .pointer("/attrs/start")
            .and_then(Value::as_i64)
            .unwrap_or(1);
        let style = node
            .pointer("/attrs/type")
            .and_then(Value::as_str)
            .unwrap_or("1");
        format!(" start=\"{start}\" type=\"{style}\"")
    } else {
        String::new()
    };
    let body = if matches!(kind(node), "paragraph" | "heading") {
        inline(node, true, notes)
    } else {
        children(node)
            .iter()
            .map(|n| html_block(n, notes))
            .collect()
    };
    format!("<{tag}{attrs}>{body}</{tag}>\n")
}
fn block(node: &Value, notes: &mut bool) -> String {
    match kind(node) {
        "paragraph" => format!("{}\n\n", inline(node, false, notes)),
        "heading" => format!(
            "{} {}\n\n",
            "#".repeat(node["attrs"]["level"].as_u64().unwrap() as usize),
            inline(node, false, notes)
        ),
        "horizontalRule" => "---\n\n".into(),
        "blockquote" => {
            let content: String = children(node).iter().map(|n| block(n, notes)).collect();
            format!(
                "{}\n\n",
                content
                    .trim_end_matches('\n')
                    .lines()
                    .map(|l| format!("> {l}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        }
        "bulletList" | "orderedList" => {
            let start = node
                .pointer("/attrs/start")
                .and_then(Value::as_i64)
                .unwrap_or(1);
            let style = node
                .pointer("/attrs/type")
                .and_then(Value::as_str)
                .unwrap_or("1");
            if kind(node) == "orderedList" && (style != "1" || start > 999_999_999) {
                *notes = true;
                return format!("{}\n", html_block(node, notes));
            }
            let mut output = String::new();
            for (index, item) in children(node).iter().enumerate() {
                let marker = if kind(node) == "bulletList" {
                    "- ".into()
                } else {
                    format!("{}. ", start + index as i64)
                };
                let content: String = children(item).iter().map(|n| block(n, notes)).collect();
                let mut lines = content.trim_end_matches('\n').split('\n');
                output.push_str(&marker);
                output.push_str(lines.next().unwrap_or(""));
                output.push('\n');
                for line in lines {
                    output.push_str(&" ".repeat(marker.len()));
                    output.push_str(line);
                    output.push('\n');
                }
                output.push('\n');
            }
            output
        }
        _ => unreachable!(),
    }
}

pub fn render_manuscript(version: i64, content: &Value) -> Result<(String, usize, bool), String> {
    let (_, count) = document_text(version, content)?;
    let mut notes = false;
    let markdown = children(content)
        .iter()
        .map(|node| block(node, &mut notes))
        .collect();
    Ok((markdown, count, notes))
}
