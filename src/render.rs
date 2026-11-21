//! A small markdown subset, parsed into styled spans.
//!
//! Width maths stays on plain text: a span carries style and an optional link,
//! and only the painter adds escape sequences. That keeps wrapping honest and
//! means a sequence can never be split across a row boundary.

/// Text from the model or a tool, without anything that could drive the
/// terminal: CSI and OSC sequences, other escapes, and control characters.
/// Newlines and tabs stay.
pub fn clean(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\n' | '\t' => out.push(c),
            '\u{1b}' => match chars.next() {
                // CSI: parameters, then one final byte.
                Some('[') => {
                    for c in chars.by_ref() {
                        if ('\u{40}'..='\u{7e}').contains(&c) {
                            break;
                        }
                    }
                }
                // OSC: up to BEL or ESC backslash.
                Some(']') => {
                    while let Some(c) = chars.next() {
                        if c == '\u{7}' || (c == '\u{1b}' && chars.next_if_eq(&'\\').is_some()) {
                            break;
                        }
                    }
                }
                _ => {}
            },
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

/// Text with colour and style codes (SGR) kept and every other escape and
/// control byte dropped, so shown output can colour itself but never move
/// the cursor, clear the screen, or draw images.
pub fn keep_sgr(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\n' => out.push(c),
            '\t' => out.push_str("    "),
            '\u{1b}' => match chars.next() {
                Some('[') => {
                    let mut seq = String::from("\u{1b}[");
                    for c in chars.by_ref() {
                        seq.push(c);
                        if ('\u{40}'..='\u{7e}').contains(&c) {
                            break;
                        }
                    }
                    // Only `m`, and only digits and separators before it.
                    if seq.ends_with('m') && seq[2..seq.len() - 1].chars().all(|c| c.is_ascii_digit() || c == ';' || c == ':') {
                        out.push_str(&seq);
                    }
                }
                // OSC, DCS (sixel), APC (kitty), PM: up to ST or BEL.
                Some(']' | 'P' | '_' | '^') => {
                    while let Some(c) = chars.next() {
                        if c == '\u{7}' || (c == '\u{1b}' && chars.next_if_eq(&'\\').is_some()) {
                            break;
                        }
                    }
                }
                _ => {}
            },
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

/// Visible characters in text that may hold SGR codes.
pub fn visible_width(text: &str) -> usize {
    let mut n = 0;
    let mut in_code = false;
    for c in text.chars() {
        match (in_code, c) {
            (false, '\u{1b}') => in_code = true,
            (true, c) if c.is_ascii_alphabetic() => in_code = false,
            (true, _) => {}
            (false, _) => n += 1,
        }
    }
    n
}

/// At most `width` visible characters, codes kept whole, ending in a reset.
pub fn fit_sgr(text: &str, width: usize) -> String {
    let mut out = String::with_capacity(text.len() + 4);
    let mut n = 0;
    let mut in_code = false;
    for c in text.chars() {
        match (in_code, c) {
            (false, '\u{1b}') => {
                in_code = true;
                out.push(c);
            }
            (true, c) => {
                out.push(c);
                in_code = !c.is_ascii_alphabetic();
            }
            (false, c) if n < width => {
                out.push(c);
                n += 1;
            }
            (false, _) => {}
        }
    }
    out.push_str("\u{1b}[0m");
    out
}

/// Rows of raw spans for shown output, each cut to `width`.
pub fn raw_rows(text: &str, width: usize) -> Vec<Vec<Span>> {
    keep_sgr(text)
        .trim_end_matches('\n')
        .split('\n')
        .map(|line| vec![Span::styled(fit_sgr(line, width), Style { raw: true, ..Style::default() })])
        .collect()
}

/// The inline attributes a run of text carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    pub code: bool,
    pub dim: bool,
    /// Table lines, painted in the border colour.
    pub border: bool,
    /// Text that carries its own colour codes, already laid out.
    pub raw: bool,
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

    let markdown = clean(markdown).replace('\t', "    ");
    let all: Vec<&str> = markdown.split('\n').collect();
    let mut at = 0;
    while at < all.len() {
        let raw = all[at];
        at += 1;
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
        if let Some(aligns) = all.get(at).and_then(|next| separator(next)).filter(|_| line.contains('|')) {
            let mut end = at + 1;
            while end < all.len() && all[end].contains('|') && !all[end].trim().is_empty() {
                end += 1;
            }
            let body: Vec<Vec<String>> = all[at + 1..end].iter().map(|l| cells(l)).collect();
            rows.extend(table(&cells(line), &aligns, &body, width, links));
            at = end;
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

#[derive(Clone, Copy, PartialEq)]
enum Align {
    Left,
    Centre,
    Right,
}

/// The cells of one table row; outer pipes are optional and `\|` is a pipe.
fn cells(line: &str) -> Vec<String> {
    let line = line.trim();
    let line = line.strip_prefix('|').unwrap_or(line);
    let line = if line.ends_with('|') && !line.ends_with("\\|") { &line[..line.len() - 1] } else { line };
    let mut out = vec![String::new()];
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&'|') => {
                chars.next();
                out.last_mut().unwrap().push('|');
            }
            '|' => out.push(String::new()),
            c => out.last_mut().unwrap().push(c),
        }
    }
    out.into_iter().map(|c| c.trim().to_string()).collect()
}

/// A separator row such as `|:---|---:|`, as the column alignments.
fn separator(line: &str) -> Option<Vec<Align>> {
    if !line.contains('-') {
        return None;
    }
    cells(line)
        .iter()
        .map(|c| {
            let core = c.trim_matches(':');
            if core.is_empty() || !core.chars().all(|ch| ch == '-') {
                return None;
            }
            Some(match (c.starts_with(':'), c.ends_with(':')) {
                (true, true) => Align::Centre,
                (false, true) => Align::Right,
                _ => Align::Left,
            })
        })
        .collect()
}

fn span_width(spans: &[Span]) -> usize {
    spans.iter().map(|s| s.text.chars().count()).sum()
}

/// A markdown table as boxed rows that fit `width`: the widest columns give
/// way first, and a table that cannot fit at all becomes `header: value` lines.
fn table(head: &[String], aligns: &[Align], body: &[Vec<String>], width: usize, links: bool) -> Vec<Vec<Span>> {
    let n = head.len();
    let bold = Style { bold: true, ..Style::default() };
    let parse = |row: &[String], style: Style| -> Vec<Vec<Span>> {
        (0..n).map(|c| inline(row.get(c).map_or("", |s| s.as_str()), style, links)).collect()
    };
    let header = parse(head, bold);
    let rows: Vec<Vec<Vec<Span>>> = body.iter().map(|r| parse(r, Style::default())).collect();
    let frame = 3 * n + 1;

    if n * 3 + frame > width {
        // Records: one `header: value` line per cell, a blank row between rows.
        let mut out = Vec::new();
        for (i, row) in rows.iter().enumerate() {
            if i > 0 {
                out.push(Vec::new());
            }
            for (c, cell) in row.iter().enumerate() {
                let mut line = vec![Span::styled(format!("{}: ", head[c]), bold)];
                line.extend(cell.iter().cloned());
                out.extend(wrap(&line, width));
            }
        }
        return out;
    }

    let mut cols: Vec<usize> = (0..n)
        .map(|c| {
            std::iter::once(&header[c])
                .chain(rows.iter().map(|r| &r[c]))
                .map(|cell| span_width(cell))
                .max()
                .unwrap_or(0)
                .max(1)
        })
        .collect();
    while cols.iter().sum::<usize>() + frame > width {
        let widest = (0..n).max_by_key(|&c| cols[c]).unwrap_or(0);
        if cols[widest] <= 3 {
            break;
        }
        cols[widest] -= 1;
    }

    let line = |text: String| Span::styled(text, Style { border: true, ..Style::default() });
    let rule = |l: &str, m: &str, r: &str| {
        let parts: Vec<String> = cols.iter().map(|w| "─".repeat(w + 2)).collect();
        vec![line(format!("{l}{}{r}", parts.join(m)))]
    };
    let draw = |cells: &[Vec<Span>]| -> Vec<Vec<Span>> {
        let wrapped: Vec<Vec<Vec<Span>>> = (0..n).map(|c| wrap(&cells[c], cols[c])).collect();
        let height = wrapped.iter().map(Vec::len).max().unwrap_or(1);
        (0..height)
            .map(|k| {
                let mut out = vec![line("│ ".into())];
                for c in 0..n {
                    let part = wrapped[c].get(k).cloned().unwrap_or_default();
                    let gap = cols[c].saturating_sub(span_width(&part));
                    let (before, after) = match aligns.get(c).copied().unwrap_or(Align::Left) {
                        Align::Left => (0, gap),
                        Align::Right => (gap, 0),
                        Align::Centre => (gap / 2, gap - gap / 2),
                    };
                    out.push(Span::plain(" ".repeat(before)));
                    out.extend(part);
                    out.push(Span::plain(" ".repeat(after)));
                    out.push(line(if c + 1 == n { " │".into() } else { " │ ".into() }));
                }
                out
            })
            .collect()
    };

    let mut out = vec![rule("┌", "┬", "┐")];
    out.extend(draw(&header));
    out.push(rule("├", "┼", "┤"));
    for row in &rows {
        out.extend(draw(row));
    }
    out.push(rule("└", "┴", "┘"));
    out
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
    fn clean_drops_escapes_and_controls() {
        assert_eq!(clean("a\u{1b}[31mred\u{1b}[0m b"), "ared b");
        assert_eq!(clean("x\u{1b}]52;c;aGk=\u{7}y"), "xy");
        assert_eq!(clean("x\u{1b}]0;title\u{1b}\\y"), "xy");
        assert_eq!(clean("50%\r100%\u{7}"), "50%100%");
        assert_eq!(clean("tab\there\nnext"), "tab\there\nnext");
        assert_eq!(clean("é ✓ 漢"), "é ✓ 漢");
    }

    fn text(row: &[Span]) -> String {
        row.iter().map(|s| s.text.as_str()).collect()
    }

    const TABLE: &str = "| Name | Size | Kind |\n|:-----|-----:|:----:|\n| raspios.img | 2.4 GB | **disk** |\n| mercury.zip | 956 MB | `zip` |";

    #[test]
    fn a_table_is_boxed_and_aligned() {
        let rows: Vec<String> = lines(TABLE, 80, true).iter().map(|r| text(r)).collect();
        assert_eq!(rows.len(), 6, "{rows:#?}");
        assert!(rows[0].starts_with('┌') && rows[0].ends_with('┐'), "{rows:#?}");
        assert!(rows[2].starts_with('├') && rows[5].starts_with('└'), "{rows:#?}");
        assert_eq!(rows[3], "│ raspios.img │ 2.4 GB │ disk │");
        assert_eq!(rows[4], "│ mercury.zip │ 956 MB │ zip  │");
        assert!(rows.iter().all(|r| r.chars().count() == rows[0].chars().count()));
        let head = &lines(TABLE, 80, true)[1];
        assert!(head.iter().any(|s| s.text == "Name" && s.style.bold));
        assert!(lines(TABLE, 80, true)[0][0].style.border);
    }

    #[test]
    fn a_wide_table_wraps_inside_its_cells() {
        let wide = "| a | b |\n|---|---|\n| one two three four five six seven | x |";
        let rows: Vec<String> = lines(wide, 20, true).iter().map(|r| text(r)).collect();
        assert!(rows.len() > 5, "{rows:#?}");
        for row in &rows {
            assert!(row.chars().count() <= 20, "{row:?}");
        }
        assert!(rows.iter().any(|r| r.contains("seven")), "{rows:#?}");
    }

    #[test]
    fn a_table_that_cannot_fit_becomes_records() {
        let rows: Vec<String> = lines(TABLE, 18, true).iter().map(|r| text(r)).collect();
        assert!(rows.contains(&"Name: raspios.img".to_string()), "{rows:#?}");
        assert!(rows.iter().any(|r| r.starts_with("Size: 956")), "{rows:#?}");
        assert!(!rows.iter().any(|r| r.contains('│')), "{rows:#?}");
    }

    #[test]
    fn table_rows_tolerate_loose_pipes() {
        let loose = "a | b\n--|--\nx \\| y | z\nshort";
        let rows: Vec<String> = lines(loose, 40, true).iter().map(|r| text(r)).collect();
        assert!(rows.iter().any(|r| r.contains("x | y") && r.contains('z')), "{rows:#?}");
        assert_eq!(rows.last().unwrap(), "short", "{rows:#?}");
        // A lone pipe without a separator is just text.
        let plain: Vec<String> = lines("a | b", 40, true).iter().map(|r| text(r)).collect();
        assert_eq!(plain, vec!["a | b"]);
    }

    #[test]
    fn a_weather_reply_draws_a_box() {
        let reply = "**Woking, UK**\n\nNow: **5°C** (feels 3°C)\n\n| Day | Low–High | Conditions | Rain |\n|---|---|---|---|\n| Today | 3–8°C | Patchy rain nearby | 29% |\n| Tomorrow | 1–8°C | Partly cloudy | 15% |";
        let rows: Vec<String> = lines(reply, 60, true).iter().map(|r| text(r)).collect();
        assert!(rows.iter().any(|r| r.contains("│ Tomorrow")), "{rows:#?}");
        assert!(!rows.iter().any(|r| r.contains("|---")), "{rows:#?}");
    }

    #[test]
    fn shown_output_keeps_colour_and_nothing_else() {
        let text = "\u{1b}[38;5;196mred\u{1b}[0m \u{1b}[2J\u{1b}[10;4Hx\u{1b}]0;title\u{7}y\u{1b}Pq#0;2;0;0;0\u{1b}\\z\u{8}";
        assert_eq!(keep_sgr(text), "\u{1b}[38;5;196mred\u{1b}[0m xyz");
        assert_eq!(visible_width("\u{1b}[1;31mab\u{1b}[0mc"), 3);
        assert_eq!(fit_sgr("\u{1b}[31mabcdef\u{1b}[0m", 3), "\u{1b}[31mabc\u{1b}[0m\u{1b}[0m");
        let rows = raw_rows("\u{1b}[32m▀▀\u{1b}[0m\nab\n", 10);
        assert_eq!(rows.len(), 2);
        assert!(rows[0][0].style.raw && rows[0][0].text.starts_with("\u{1b}[32m▀▀"));
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
