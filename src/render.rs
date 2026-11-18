//! A small markdown subset, parsed into styled spans.
//!
//! Width maths stays on plain text: a span carries style and an optional link,
//! and only the painter adds escape sequences. That keeps wrapping honest and
//! means a sequence can never be split across a row boundary.

/// The inline attributes a run of text carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    pub code: bool,
    pub dim: bool,
}

/// A run of text with one style and at most one link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    pub style: Style,
    pub link: Option<String>,
}

impl Span {
    fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: Style::default(),
            link: None,
        }
    }

    fn styled(text: impl Into<String>, style: Style) -> Self {
        Self {
            text: text.into(),
            style,
            link: None,
        }
    }
}

/// Render `markdown` into rows of spans, each no wider than `width`.
///
/// `links` decides whether a link renders as its text or as `text (url)`, so the
/// width is right either way.
pub fn lines(markdown: &str, width: usize, links: bool) -> Vec<Vec<Span>> {
    let width = width.max(8);
    let mut rows = Vec::new();
    let mut fence: Option<char> = None;
    let mut fence_body: Vec<String> = Vec::new();

    for raw in markdown.split('\n') {
        let line = raw.trim_end();
        if let Some(marker) = fence {
            if is_fence(line, marker) {
                for body in std::mem::take(&mut fence_body) {
                    rows.extend(hard_wrap(
                        &[Span::styled(body, Style { dim: true, ..Style::default() })],
                        width,
                    ));
                }
                fence = None;
            } else {
                fence_body.push(line.to_string());
            }
            continue;
        }
        if let Some(marker) = fence_start(line) {
            fence = Some(marker);
            continue;
        }

        let (prefix, rest, style) = block(line);
        let indent = prefix.chars().count();
        let body = inline(rest, style, links);
        if body.iter().all(|s| s.text.trim().is_empty()) {
            if indent > 0 {
                rows.push(vec![Span::styled(prefix, style)]);
            } else if line.trim().is_empty() {
                rows.push(Vec::new());
            }
            continue;
        }
        let room = width.saturating_sub(indent).max(1);
        for (i, mut row) in wrap(&body, room).into_iter().enumerate() {
            if indent > 0 {
                let lead = if i == 0 {
                    Span::styled(prefix.clone(), style)
                } else {
                    Span::plain(" ".repeat(indent))
                };
                row.insert(0, lead);
            }
            rows.push(row);
        }
    }

    // An unterminated fence is still shown as code, which is what the model meant.
    for body in fence_body {
        rows.extend(hard_wrap(
            &[Span::styled(body, Style { dim: true, ..Style::default() })],
            width,
        ));
    }
    rows
}

fn fence_start(line: &str) -> Option<char> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("```") {
        Some('`')
    } else if trimmed.starts_with("~~~") {
        Some('~')
    } else {
        None
    }
}

fn is_fence(line: &str, marker: char) -> bool {
    let trimmed = line.trim_start();
    let marks: String = std::iter::repeat(marker).take(3).collect();
    trimmed.starts_with(&marks)
}

/// The marker, the remaining text, and the style the whole row takes.
fn block(line: &str) -> (String, &str, Style) {
    let dim = Style { dim: true, ..Style::default() };
    if line.trim().is_empty() {
        return (String::new(), line, Style::default());
    }
    if line.trim() == "---" || line.trim() == "***" {
        return ("─".repeat(8), "", dim);
    }
    if let Some(rest) = line.strip_prefix("# ") {
        return (String::new(), rest, Style { bold: true, ..Style::default() });
    }
    if let Some(rest) = line.strip_prefix("## ") {
        return (String::new(), rest, Style { bold: true, ..Style::default() });
    }
    if let Some(rest) = line.strip_prefix("### ") {
        return (String::new(), rest, Style { bold: true, ..Style::default() });
    }
    if let Some(rest) = line.strip_prefix("> ") {
        return ("│ ".into(), rest, dim);
    }
    if line == ">" {
        return ("│".into(), "", dim);
    }
    if let Some(rest) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
        return ("• ".into(), rest, Style::default());
    }
    let digits: String = line.chars().take_while(|c| c.is_ascii_digit()).collect();
    if !digits.is_empty() {
        if let Some(rest) = line[digits.len()..].strip_prefix(". ") {
            return (format!("{digits}. "), rest, Style::default());
        }
    }
    (String::new(), line, Style::default())
}

/// Inline styles: bold, italic, code, markdown links, and bare URLs.
fn inline(text: &str, base: Style, links: bool) -> Vec<Span> {
    let chars: Vec<char> = text.chars().collect();
    let mut spans: Vec<Span> = Vec::new();
    let mut plain = String::new();
    let mut i = 0;

    let flush = |spans: &mut Vec<Span>, plain: &mut String, style: Style| {
        if !plain.is_empty() {
            spans.push(Span::styled(std::mem::take(plain), style));
        }
    };

    while i < chars.len() {
        let c = chars[i];

        if c == '`' {
            if let Some(end) = chars[i + 1..].iter().position(|&x| x == '`') {
                flush(&mut spans, &mut plain, base);
                let body: String = chars[i + 1..i + 1 + end].iter().collect();
                spans.push(Span::styled(
                    body,
                    Style { code: true, ..base },
                ));
                i += end + 2;
                continue;
            }
        }

        if c == '*' && chars.get(i + 1) == Some(&'*') {
            if let Some(end) = find_double(&chars, i + 2) {
                flush(&mut spans, &mut plain, base);
                let body: String = chars[i + 2..end].iter().collect();
                spans.push(Span::styled(
                    body,
                    Style { bold: true, ..base },
                ));
                i = end + 2;
                continue;
            }
        }

        if c == '*' || c == '_' {
            if let Some(end) = chars[i + 1..].iter().position(|&x| x == c) {
                if end > 0 {
                    flush(&mut spans, &mut plain, base);
                    let body: String = chars[i + 1..i + 1 + end].iter().collect();
                    spans.push(Span::styled(
                        body,
                        Style { italic: true, ..base },
                    ));
                    i += end + 2;
                    continue;
                }
            }
        }

        if c == '[' {
            if let Some((label, url, next)) = parse_link(&chars, i) {
                flush(&mut spans, &mut plain, base);
                if links {
                    spans.push(Span {
                        text: label.clone(),
                        style: base,
                        link: Some(url.clone()),
                    });
                } else {
                    spans.push(Span::styled(label, base));
                    spans.push(Span::styled(format!(" ({url})"), Style { dim: true, ..base }));
                }
                i = next;
                continue;
            }
        }

        if c == 'h' && text_at(&chars, i, "https://") {
            let end = chars[i..]
                .iter()
                .position(|&x| x.is_whitespace() || x == ')' || x == ']')
                .map(|n| i + n)
                .unwrap_or(chars.len());
            flush(&mut spans, &mut plain, base);
            let url: String = chars[i..end].iter().collect();
            if links {
                spans.push(Span {
                    text: url.clone(),
                    style: base,
                    link: Some(url),
                });
            } else {
                spans.push(Span::styled(url, Style { dim: true, ..base }));
            }
            i = end;
            continue;
        }

        plain.push(c);
        i += 1;
    }
    flush(&mut spans, &mut plain, base);
    spans
}

fn text_at(chars: &[char], i: usize, needle: &str) -> bool {
    let n: Vec<char> = needle.chars().collect();
    chars.len() >= i + n.len() && chars[i..i + n.len()] == n[..]
}

fn find_double(chars: &[char], from: usize) -> Option<usize> {
    let mut i = from;
    while i + 1 < chars.len() {
        if chars[i] == '*' && chars[i + 1] == '*' {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// `[label](url)` starting at `open`; returns the label, url, and the next index.
fn parse_link(chars: &[char], open: usize) -> Option<(String, String, usize)> {
    let mut i = open + 1;
    let mut label = String::new();
    while i < chars.len() && chars[i] != ']' {
        label.push(chars[i]);
        i += 1;
    }
    if i >= chars.len() || chars.get(i + 1) != Some(&'(') {
        return None;
    }
    i += 2;
    let mut url = String::new();
    while i < chars.len() && chars[i] != ')' {
        url.push(chars[i]);
        i += 1;
    }
    if i >= chars.len() || label.is_empty() || url.is_empty() {
        return None;
    }
    Some((label, url, i + 1))
}

/// Greedy word wrap across spans, keeping each word's style and link.
pub fn wrap(spans: &[Span], width: usize) -> Vec<Vec<Span>> {
    let mut rows: Vec<Vec<Span>> = Vec::new();
    let mut row: Vec<Span> = Vec::new();
    let mut used = 0;

    for span in spans {
        for piece in split_words(&span.text) {
            if piece.is_empty() {
                continue;
            }
            let len = piece.chars().count();
            let space = if used == 0 { 0 } else { 1 };
            if used + space + len > width && used > 0 {
                rows.push(std::mem::take(&mut row));
                used = 0;
            }
            if len > width {
                for chunk in chunk(&piece, width) {
                    if used > 0 {
                        rows.push(std::mem::take(&mut row));
                        used = 0;
                    }
                    rows.push(vec![Span {
                        text: chunk.clone(),
                        style: span.style,
                        link: span.link.clone(),
                    }]);
                }
                continue;
            }
            if used > 0 {
                row.push(Span::plain(" "));
                used += 1;
            }
            row.push(Span {
                text: piece,
                style: span.style,
                link: span.link.clone(),
            });
            used += len;
        }
    }
    if !row.is_empty() {
        rows.push(row);
    }
    if rows.is_empty() {
        rows.push(Vec::new());
    }
    rows
}

/// Hard wrap, for code where a long line should break rather than disappear.
fn hard_wrap(spans: &[Span], width: usize) -> Vec<Vec<Span>> {
    let mut rows = Vec::new();
    for span in spans {
        if span.text.chars().count() <= width {
            rows.push(vec![span.clone()]);
            continue;
        }
        for chunk in chunk(&span.text, width) {
            rows.push(vec![Span {
                text: chunk,
                style: span.style,
                link: span.link.clone(),
            }]);
        }
    }
    rows
}

fn split_words(text: &str) -> Vec<String> {
    text.split(' ').map(|w| w.to_string()).collect()
}

fn chunk(text: &str, width: usize) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    chars
        .chunks(width)
        .map(|c| c.iter().collect::<String>())
        .collect()
}

/// The OSC 8 hyperlink wrapper. Terminals that do not know it show the text.
pub fn hyperlink(url: &str, text: &str) -> String {
    format!("\u{1b}]8;;{url}\u{1b}\\{text}\u{1b}]8;;\u{1b}\\")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(rows: &[Vec<Span>]) -> Vec<String> {
        rows.iter()
            .map(|r| r.iter().map(|s| s.text.as_str()).collect::<String>())
            .collect()
    }

    #[test]
    fn bold_and_italic_and_code() {
        let rows = lines("**done** in *soon* with `imp`", 80, false);
        let spans = &rows[0];
        assert!(spans.iter().any(|s| s.text == "done" && s.style.bold));
        assert!(spans.iter().any(|s| s.text == "soon" && s.style.italic));
        assert!(spans.iter().any(|s| s.text == "imp" && s.style.code));
    }

    #[test]
    fn a_half_written_link_stays_text() {
        let rows = lines("see [docs](ht", 80, true);
        assert_eq!(plain(&rows), vec!["see [docs](ht"]);
        assert!(rows[0].iter().all(|s| s.link.is_none()));
    }

    #[test]
    fn links_render_as_text_or_text_and_url() {
        let on = lines("see [docs](https://x.dev/a)", 80, true);
        let span = on[0].iter().find(|s| s.text == "docs").expect("a link");
        assert_eq!(span.link.as_deref(), Some("https://x.dev/a"));

        let off = lines("see [docs](https://x.dev/a)", 80, false);
        assert_eq!(
            plain(&off),
            vec!["see docs (https://x.dev/a)"]
        );
    }

    #[test]
    fn bare_urls_are_linked() {
        let rows = lines("read https://x.dev/a today", 80, true);
        assert!(rows[0].iter().any(|s| s.link.as_deref() == Some("https://x.dev/a")));
    }

    #[test]
    fn a_fence_keeps_its_body() {
        let text = "before\n```\n**not bold**\n```\nafter";
        let rows = lines(text, 80, false);
        let joined = plain(&rows);
        assert!(joined.iter().any(|r| r == "**not bold**"));
        assert!(rows[1][0].style.dim, "the fence body should be dim");
        assert!(!rows[1][0].style.code, "a fence body is not inline code");
    }

    #[test]
    fn blocks_take_a_marker() {
        let rows = lines("- one\n- two\n> quoted\n# title", 80, false);
        let joined = plain(&rows);
        assert_eq!(joined[0], "• one");
        assert_eq!(joined[1], "• two");
        assert_eq!(joined[2], "│ quoted");
        assert_eq!(joined[3], "title");
    }

    #[test]
    fn wrapping_keeps_the_width() {
        let rows = lines("alpha **beta gamma** delta epsilon", 12, false);
        for row in &rows {
            let len: usize = row.iter().map(|s| s.text.chars().count()).sum();
            assert!(len <= 12, "{row:?} is {len}");
        }
        assert!(rows.len() > 1);
    }

    #[test]
    fn a_list_item_indents_its_wrapped_rows() {
        let rows = lines("- alpha beta gamma delta epsilon", 12, false);
        assert!(rows.len() > 1, "no wrap: {rows:?}");
        for row in &rows[1..] {
            let text: String = row.iter().map(|s| s.text.as_str()).collect();
            assert!(text.starts_with("  "), "continuation not indented: {text:?}");
        }
    }

    #[test]
    fn a_link_survives_a_wrap() {
        let rows = lines("[one two three four five](https://x.dev)", 9, true);
        assert!(rows.len() > 1);
        for row in &rows {
            if let Some(span) = row.iter().find(|s| s.link.is_some()) {
                assert_eq!(span.link.as_deref(), Some("https://x.dev"));
            }
        }
    }
}
