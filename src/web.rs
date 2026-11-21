//! What `fetch` hands the model: HTML as compact markdown, feeds as lists.

/// Elements whose whole content is dropped: code, chrome, and page furniture.
const SKIP: &[&str] = &[
    "script", "style", "noscript", "head", "title", "nav", "footer", "aside", "form", "svg", "template", "iframe",
    "button", "select",
];

/// An HTML page as markdown: the title as a heading, then headings, links with
/// absolute URLs, list items, and table rows. `base` resolves relative links.
// ponytail: a tag walker, not a parser; malformed pages degrade to plain text.
pub fn markdown(html: &str, base: &str) -> String {
    let lower = html.to_ascii_lowercase();
    let mut out = String::new();
    if let Some(title) = between(html, &lower, "<title", "</title") {
        let title = collapse(&decode(&title));
        if !title.is_empty() {
            out.push_str(&format!("# {title}\n\n"));
        }
    }
    // The page's own content, when it marks it: banners and menus sit outside.
    let (mut at, stop) = match lower.find("<main") {
        Some(start) => (start, lower[start..].find("</main").map_or(html.len(), |i| start + i)),
        None => (0, html.len()),
    };
    let (html, lower) = (&html[..stop], &lower[..stop]);
    // Open links: where their text starts in `out`, and their target.
    let mut links: Vec<(usize, String)> = Vec::new();
    while let Some(open) = lower[at..].find('<').map(|i| i + at) {
        push_text(&mut out, &html[at..open]);
        let Some(close) = tag_end(lower, open) else {
            at = html.len();
            break;
        };
        let raw = &html[open + 1..close];
        let tag = &lower[open + 1..close];
        at = close + 1;
        if tag.starts_with("!--") {
            at = lower[open..].find("-->").map_or(html.len(), |i| open + i + 3);
            continue;
        }
        let closing = tag.starts_with('/');
        let name: String = tag.trim_start_matches('/').chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
        if !closing && SKIP.contains(&name.as_str()) && !tag.ends_with('/') {
            let end = format!("</{name}");
            at = lower[at..].find(&end).map_or(html.len(), |i| i + at);
            continue;
        }
        if !closing && !tag.ends_with('/') && chrome(tag) {
            at = matching_close(lower, at, &name);
            continue;
        }
        match (name.as_str(), closing) {
            ("h1" | "h2" | "h3" | "h4" | "h5" | "h6", false) => {
                let level = name[1..].parse().unwrap_or(1);
                out.push_str(&format!("\n\n{} ", "#".repeat(level)));
            }
            ("h1" | "h2" | "h3" | "h4" | "h5" | "h6", true) => out.push_str("\n\n"),
            ("li", false) => out.push_str("\n- "),
            ("tr", false) => out.push_str("\n|"),
            ("td" | "th", true) => out.push_str(" |"),
            ("td" | "th", false) => out.push(' '),
            ("br", _) => out.push('\n'),
            ("p" | "div" | "section" | "article" | "main" | "header" | "table" | "ul" | "ol" | "blockquote"
            | "dl" | "dt" | "dd" | "figure" | "pre", _) => out.push_str("\n\n"),
            ("a", false) => links.push((out.len(), absolute(base, &attr(raw, "href").unwrap_or_default()))),
            ("a", true) => {
                if let Some((start, href)) = links.pop() {
                    let text = collapse(&out[start..]);
                    out.truncate(start);
                    match (text.is_empty(), href.is_empty()) {
                        (true, _) => {}
                        (false, true) => out.push_str(&text),
                        (false, false) => out.push_str(&format!("[{text}]({href})")),
                    }
                }
            }
            _ => {}
        }
    }
    if at < html.len() {
        push_text(&mut out, &html[at..]);
    }
    tidy(&out)
}

/// An RSS or Atom feed as a markdown list of title, date, and link, or `None`
/// when the body is not a feed.
pub fn feed(xml: &str) -> Option<String> {
    let lower = xml.to_ascii_lowercase();
    let head = &lower[..lower.len().min(2048)];
    let (item, end) = if head.contains("<rss") || head.contains("<rdf:rdf") {
        ("<item", "</item>")
    } else if head.contains("<feed") {
        ("<entry", "</entry>")
    } else {
        return None;
    };
    let mut out = String::new();
    let first = lower.find(item).unwrap_or(lower.len());
    if let Some(title) = between(&xml[..first], &lower[..first], "<title", "</title") {
        out.push_str(&format!("# {}\n\n", text_of(&title)));
    }
    let mut at = first;
    while let Some(start) = lower[at..].find(item).map(|i| i + at) {
        let stop = lower[start..].find(end).map_or(xml.len(), |i| start + i);
        let (raw, low) = (&xml[start..stop], &lower[start..stop]);
        let title = between(raw, low, "<title", "</title").map(|t| text_of(&t)).unwrap_or_default();
        let link = between(raw, low, "<link", "</link")
            .map(|t| text_of(&t))
            .filter(|l| !l.is_empty())
            .or_else(|| low.find("<link").and_then(|i| attr(&raw[i..raw[i..].find('>').map_or(raw.len(), |j| i + j)], "href")))
            .map(|l| untracked(&l))
            .unwrap_or_default();
        let date = ["<pubdate", "<updated", "<published", "<dc:date"]
            .iter()
            .find_map(|tag| between(raw, low, tag, &tag.replacen('<', "</", 1)))
            .map(|d| text_of(&d))
            .unwrap_or_default();
        let mut line = format!("- {title}");
        if !date.is_empty() {
            line.push_str(&format!(" · {date}"));
        }
        if !link.is_empty() {
            line.push_str(&format!(" · {link}"));
        }
        out.push_str(&line);
        out.push('\n');
        at = stop + end.len().min(xml.len() - stop);
    }
    Some(out.trim_end().to_string())
}

/// The `>` that ends the tag opening at `open`, skipping quoted values, which
/// may hold `>` themselves.
fn tag_end(lower: &str, open: usize) -> Option<usize> {
    let mut quote = None;
    for (i, b) in lower.bytes().enumerate().skip(open + 1) {
        match (quote, b) {
            (None, b'"' | b'\'') => quote = Some(b),
            (Some(q), _) if b == q => quote = None,
            (None, b'>') => return Some(i),
            // A stray `<` before any close: treat the first one as text.
            (None, b'<') => return lower[open..i].find('>').map(|j| open + j),
            _ => {}
        }
    }
    None
}

/// Words in a class, id, or role that mark page furniture rather than content.
const CHROME: &[&str] = &[
    "dropdown", "menu", "nav", "navbar", "navigation", "sidebar", "cookie", "cookies", "banner", "breadcrumb",
    "breadcrumbs", "share", "social", "toc", "advert", "ads", "newsletter", "subscribe", "popup",
    "modal", "contentinfo", "search",
];

/// A tag whose class, id, or role names furniture. Utility classes with
/// arbitrary values, such as `scroll-mt-[calc(var(--nav-height))]`, describe
/// layout, not purpose, so they never count.
fn chrome(tag: &str) -> bool {
    ["class", "id", "role"].iter().filter_map(|a| attr(tag, a)).any(|v| {
        v.split_whitespace()
            .filter(|name| !name.contains(['[', '(', ':']))
            .any(|name| name.split(|c: char| !c.is_ascii_alphanumeric()).any(|w| CHROME.contains(&w)))
    })
}

/// Just past the close of the element whose open tag ends at `from`,
/// counting nested elements of the same name.
fn matching_close(lower: &str, from: usize, name: &str) -> usize {
    let (open, close) = (format!("<{name}"), format!("</{name}"));
    let mut depth = 1;
    let mut at = from;
    while depth > 0 {
        let next_open = lower[at..].find(&open).map(|i| i + at);
        let Some(next_close) = lower[at..].find(&close).map(|i| i + at) else { return lower.len() };
        match next_open {
            // Only a real tag start counts, not `<divider` for `<div`.
            Some(o) if o < next_close => {
                let after = lower[o + open.len()..].chars().next();
                if after.is_some_and(|c| c == '>' || c.is_whitespace() || c == '/') {
                    depth += 1;
                }
                at = o + open.len();
            }
            _ => {
                depth -= 1;
                at = next_close + close.len();
            }
        }
    }
    lower[at..].find('>').map_or(lower.len(), |i| at + i + 1)
}

/// The text inside `<open …>…</close`, matched in `lower`, sliced from `raw`.
fn between(raw: &str, lower: &str, open: &str, close: &str) -> Option<String> {
    let at = lower.find(open)?;
    let start = lower[at..].find('>')? + at + 1;
    let end = lower[start..].find(close)? + start;
    Some(raw[start..end].to_string())
}

/// Element text without CDATA wrappers or tags, entities decoded.
fn text_of(raw: &str) -> String {
    let raw = raw.trim();
    let raw = raw.strip_prefix("<![CDATA[").and_then(|r| r.strip_suffix("]]>")).unwrap_or(raw);
    let mut plain = String::new();
    let mut inside = false;
    for c in raw.chars() {
        match c {
            '<' => inside = true,
            '>' => inside = false,
            c if !inside => plain.push(c),
            _ => {}
        }
    }
    collapse(&decode(&plain))
}

/// A quoted or bare attribute value from a raw tag.
fn attr(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let mut from = 0;
    while let Some(i) = lower[from..].find(name).map(|i| i + from) {
        from = i + name.len();
        let before_ok = i == 0 || lower.as_bytes()[i - 1].is_ascii_whitespace();
        let rest = lower[from..].trim_start();
        if !before_ok || !rest.starts_with('=') {
            continue;
        }
        let value = tag[tag.len() - rest.len() + 1..].trim_start();
        let v = match value.chars().next() {
            Some(q @ ('"' | '\'')) => value[1..].split(q).next().unwrap_or(""),
            _ => value.split(|c: char| c.is_whitespace() || c == '>').next().unwrap_or(""),
        };
        return Some(decode(v));
    }
    None
}

/// A link target as an absolute URL, or empty for fragments and scripts.
fn absolute(base: &str, href: &str) -> String {
    let href = href.trim();
    let lower = href.to_ascii_lowercase();
    if href.is_empty() || href.starts_with('#') || lower.starts_with("javascript:") {
        return String::new();
    }
    if lower.starts_with("mailto:") {
        return href.to_string();
    }
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return untracked(href);
    }
    let (scheme, rest) = base.split_once("://").unwrap_or(("https", base));
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    if let Some(path) = href.strip_prefix("//") {
        return format!("{scheme}://{path}");
    }
    if href.starts_with('/') {
        return untracked(&format!("{scheme}://{host}{href}"));
    }
    let path = rest[host.len()..].split(['?', '#']).next().unwrap_or("");
    let dir = path.rfind('/').map_or("/", |i| &path[..=i]);
    untracked(&format!("{scheme}://{host}{dir}{href}"))
}

/// A URL without campaign-tracking parameters, which only cost tokens.
fn untracked(url: &str) -> String {
    let Some((head, query)) = url.split_once('?') else { return url.to_string() };
    let (query, frag) = query.split_once('#').map_or((query, None), |(q, f)| (q, Some(f)));
    let kept: Vec<&str> = query
        .split('&')
        .filter(|p| !p.is_empty() && !p.starts_with("utm_") && !p.starts_with("at_") && !p.starts_with("fbclid="))
        .collect();
    let mut out = head.to_string();
    if !kept.is_empty() {
        out.push('?');
        out.push_str(&kept.join("&"));
    }
    if let Some(f) = frag {
        out.push('#');
        out.push_str(f);
    }
    out
}

/// Append a text node with HTML whitespace folded to single spaces.
fn push_text(out: &mut String, text: &str) {
    if text.is_empty() {
        return;
    }
    let decoded = decode(text);
    let mut last_space = out.ends_with([' ', '\n']) || out.is_empty();
    for c in decoded.chars() {
        if c.is_whitespace() {
            if !last_space {
                out.push(' ');
            }
            last_space = true;
        } else {
            out.push(c);
            last_space = false;
        }
    }
}

/// Lines trimmed, empty bullets and rows dropped, a line repeating the one
/// before it dropped, as a title repeated by the first heading is, and one
/// blank line at most.
fn tidy(text: &str) -> String {
    let mut out = String::new();
    let mut blank = true;
    let mut last = "";
    for line in text.lines().map(str::trim) {
        if !line.is_empty() && line == last {
            continue;
        }
        let empty = line.is_empty() || line == "-" || line.chars().all(|c| c == '|' || c == ' ');
        if !empty {
            last = line;
        }
        if empty {
            if !blank {
                out.push('\n');
            }
            blank = true;
        } else {
            out.push_str(line);
            out.push('\n');
            blank = false;
        }
    }
    out.trim_end().to_string()
}

fn collapse(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Named and numeric character references.
fn decode(text: &str) -> String {
    if !text.contains('&') {
        return text.to_string();
    }
    const NAMED: &[(&str, &str)] = &[
        ("nbsp", " "), ("amp", "&"), ("lt", "<"), ("gt", ">"), ("quot", "\""), ("apos", "'"),
        ("rsquo", "’"), ("lsquo", "‘"), ("rdquo", "”"), ("ldquo", "“"), ("hellip", "…"),
        ("ndash", "–"), ("mdash", "—"), ("pound", "£"), ("euro", "€"), ("copy", "©"), ("deg", "°"),
    ];
    let mut out = String::new();
    let mut rest = text;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let end = rest[1..].find(';').map(|j| j + 1).filter(|&j| j <= 10);
        let decoded = end.and_then(|j| {
            let name = &rest[1..j];
            let code = name.strip_prefix("#x").or_else(|| name.strip_prefix("#X"));
            if let Some(hex) = code {
                u32::from_str_radix(hex, 16).ok().and_then(char::from_u32).map(String::from)
            } else if let Some(dec) = name.strip_prefix('#') {
                dec.parse().ok().and_then(char::from_u32).map(String::from)
            } else {
                NAMED.iter().find(|(n, _)| *n == name).map(|(_, v)| v.to_string())
            }
            .map(|s| (s, j))
        });
        match decoded {
            Some((s, j)) => {
                out.push_str(&s);
                rest = &rest[j + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_becomes_compact_markdown() {
        let page = "<html><head><title>Moon &amp; Sun</title><style>p{}</style></head><body>\
            <nav><a href='/'>Home</a></nav><script>alert(1)</script>\
            <h1>Phase</h1><p>Waxing&nbsp;gibbous, see <a href=\"/moon/full?x=1\">full &rarr;moon</a>.</p>\
            <ul><li>New</li><li><a href='#top'>Top</a></li></ul>\
            <table><tr><th>Day</th><th>Rise</th></tr><tr><td>Sat</td><td>07:12</td></tr></table>\
            <p>Next: <b>full</b> &lt;3 days&gt; &#8211; &#x2014;</p><footer>© 2026</footer></body></html>";
        let md = markdown(page, "https://sky.example/today/index.html");
        let want = "# Moon & Sun\n\n# Phase\n\nWaxing gibbous, see [full &rarr;moon](https://sky.example/moon/full?x=1).\n\n\
            - New\n- Top\n\n| Day | Rise |\n| Sat | 07:12 |\n\nNext: full <3 days> – —";
        assert_eq!(md, want);
        assert!(!md.contains("Home") && !md.contains("alert") && !md.contains("2026"));
    }

    #[test]
    fn a_title_repeated_as_the_first_heading_shows_once() {
        assert_eq!(markdown("<title>Results</title><h1>Results</h1><p>x</p>", "https://w.example/"), "# Results\n\nx");
    }

    #[test]
    fn main_content_wins_and_trackers_go() {
        let page = "<title>T</title><div>Cookies on this site</div><main><p>Body \
            <a href='https://x.example/a?utm_source=rss&id=7&at_medium=RSS'>link</a></p></main><p>after</p>";
        assert_eq!(markdown(page, "https://x.example/"), "# T\n\nBody [link](https://x.example/a?id=7)");
        assert_eq!(untracked("https://x.example/a?at_medium=RSS&at_campaign=rss"), "https://x.example/a");
        assert_eq!(untracked("https://x.example/a?q=1#s"), "https://x.example/a?q=1#s");
    }

    #[test]
    fn a_quoted_angle_bracket_does_not_end_a_tag() {
        let page = "<p data-mw='{\"x\":\"<!-- a -->\"}'>Town</p><p>b > c</p>";
        assert_eq!(markdown(page, "https://w.example/"), "Town\n\nb > c");
    }

    #[test]
    fn page_furniture_is_dropped() {
        let page = "<main><div class='vector-dropdown mw-portlet-lang'><div><a href='/af'>Afrikaans</a></div></div>\
            <ul role='navigation'><li>Menu</li></ul><p>Woking is a town. In stock.</p>\
            <div id='toc'><div>Contents</div></div><div class='content'>More</div></main>";
        assert_eq!(markdown(page, "https://w.example/"), "Woking is a town. In stock.\n\nMore");
        let tailwind = "<main><div class='scroll-mt-[calc(var(--f1-nav-height))] p-4'><table><tr><td>Norris</td></tr></table></div></main>";
        assert_eq!(markdown(tailwind, "https://w.example/"), "| Norris |");
    }

    #[test]
    fn relative_links_resolve_against_the_page() {
        let b = "https://h.example/a/b.html?q=1";
        assert_eq!(absolute(b, "c.html"), "https://h.example/a/c.html");
        assert_eq!(absolute(b, "/d"), "https://h.example/d");
        assert_eq!(absolute(b, "//cdn.example/e"), "https://cdn.example/e");
        assert_eq!(absolute(b, "https://x.example/"), "https://x.example/");
        assert_eq!(absolute(b, "#frag"), "");
        assert_eq!(absolute(b, "javascript:void(0)"), "");
        assert_eq!(absolute("https://h.example", "f"), "https://h.example/f");
    }

    #[test]
    fn rss_becomes_a_list() {
        let xml = r#"<?xml version="1.0"?><rss><channel><title><![CDATA[BBC News]]></title>
            <item><title><![CDATA[Deal with EU agreed]]></title><link>https://bbc.example/1</link>
            <pubDate>Sat, 21 Nov 2026 09:12:00 GMT</pubDate></item>
            <item><title>Markets &amp; AI</title><link>https://bbc.example/2</link></item></channel></rss>"#;
        assert_eq!(
            feed(xml).unwrap(),
            "# BBC News\n\n- Deal with EU agreed · Sat, 21 Nov 2026 09:12:00 GMT · https://bbc.example/1\n\
             - Markets & AI · https://bbc.example/2"
        );
    }

    #[test]
    fn atom_becomes_a_list() {
        let xml = r#"<feed xmlns="http://www.w3.org/2005/Atom"><title>Rust Blog</title>
            <entry><title>Announcing Rust 1.100.0</title><link rel="alternate" href="https://blog.example/r"/>
            <updated>2026-11-09T00:00:00Z</updated></entry></feed>"#;
        assert_eq!(
            feed(xml).unwrap(),
            "# Rust Blog\n\n- Announcing Rust 1.100.0 · 2026-11-09T00:00:00Z · https://blog.example/r"
        );
    }

    #[test]
    fn other_xml_is_not_a_feed() {
        assert!(feed("<svg><title>x</title></svg>").is_none());
        assert!(feed("{\"a\":1}").is_none());
    }
}
