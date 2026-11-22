//! Syntax kinds for diff rows, by file extension. Hand-written rules, one
//! left-to-right pass per line; a string or comment spanning lines is
//! styled on its first line only.

/// The kinds a character can have. Zero is plain.
pub const KEYWORD: u8 = 1;
pub const STRING: u8 = 2;
pub const NUMBER: u8 = 3;
pub const COMMENT: u8 = 4;

struct Rules {
    keywords: &'static [&'static str],
    comment: &'static str,
    quotes: &'static str,
}

const RUST: Rules = Rules {
    keywords: &[
        "as", "break", "const", "continue", "crate", "else", "enum", "fn", "for", "if", "impl", "in", "let", "loop",
        "match", "mod", "move", "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "trait", "type",
        "unsafe", "use", "where", "while",
    ],
    comment: "//",
    // `'` starts a lifetime as often as a character, so only `"` quotes.
    quotes: "\"",
};
const SHELL: Rules = Rules {
    keywords: &[
        "if", "then", "else", "elif", "fi", "for", "while", "do", "done", "case", "esac", "in", "function", "return",
        "local", "export",
    ],
    comment: "#",
    quotes: "\"'",
};
const PYTHON: Rules = Rules {
    keywords: &[
        "and", "as", "class", "def", "elif", "else", "except", "False", "for", "from", "if", "import", "in", "is",
        "lambda", "None", "not", "or", "pass", "raise", "return", "True", "try", "while", "with", "yield",
    ],
    comment: "#",
    quotes: "\"'",
};
const SCRIPT: Rules = Rules {
    keywords: &[
        "async", "await", "break", "case", "class", "const", "continue", "default", "else", "export", "extends", "for",
        "from", "function", "if", "import", "let", "new", "return", "switch", "this", "throw", "try", "typeof", "var",
        "while",
    ],
    comment: "//",
    quotes: "\"'`",
};
const TOML: Rules = Rules { keywords: &["true", "false"], comment: "#", quotes: "\"'" };
const YAML: Rules = Rules { keywords: &["true", "false", "null"], comment: "#", quotes: "\"'" };

fn rules(ext: &str) -> Option<&'static Rules> {
    Some(match ext {
        "rs" => &RUST,
        "sh" | "bash" | "zsh" => &SHELL,
        "py" => &PYTHON,
        "js" | "mjs" | "ts" | "tsx" | "jsx" => &SCRIPT,
        "toml" => &TOML,
        "yml" | "yaml" => &YAML,
        _ => return None,
    })
}

/// One kind per character of `line` for a file with extension `ext`: all
/// plain for an extension without rules.
pub fn kinds(ext: &str, line: &str) -> Vec<u8> {
    let chars: Vec<char> = line.chars().collect();
    let n = chars.len();
    let mut out = vec![0u8; n];
    let Some(r) = rules(ext) else { return out };
    let word = |c: char| c.is_alphanumeric() || c == '_';
    let comment: Vec<char> = r.comment.chars().collect();
    let mut i = 0;
    while i < n {
        let c = chars[i];
        // `#` opens a comment only after a space or at the start, so `$#`
        // and `x#y` stay code.
        let at_comment = chars[i..].starts_with(&comment) && (comment != ['#'] || i == 0 || chars[i - 1].is_whitespace());
        if at_comment {
            out[i..].fill(COMMENT);
            break;
        }
        let end = if r.quotes.contains(c) {
            let mut j = i + 1;
            while j < n && chars[j] != c {
                j += if chars[j] == '\\' { 2 } else { 1 };
            }
            let end = (j + 1).min(n);
            out[i..end].fill(STRING);
            end
        } else if c.is_ascii_digit() && (i == 0 || !word(chars[i - 1])) {
            let end = (i..n).find(|&j| !(word(chars[j]) || chars[j] == '.')).unwrap_or(n);
            out[i..end].fill(NUMBER);
            end
        } else if word(c) {
            let end = (i..n).find(|&j| !word(chars[j])).unwrap_or(n);
            let w: String = chars[i..end].iter().collect();
            if r.keywords.contains(&w.as_str()) {
                out[i..end].fill(KEYWORD);
            }
            end
        } else {
            i + 1
        };
        i = end;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The text of each run of one kind, in order.
    fn runs(ext: &str, line: &str, kind: u8) -> Vec<String> {
        let k = kinds(ext, line);
        let mut out = Vec::new();
        let mut cur = String::new();
        for (c, m) in line.chars().zip(&k) {
            if *m == kind {
                cur.push(c);
            } else if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
        }
        if !cur.is_empty() {
            out.push(cur);
        }
        out
    }

    #[test]
    fn rust() {
        let line = r#"let x = "hi"; // note"#;
        assert_eq!(runs("rs", line, KEYWORD), vec!["let"]);
        assert_eq!(runs("rs", line, STRING), vec!["\"hi\""]);
        assert_eq!(runs("rs", line, COMMENT), vec!["// note"]);
        assert_eq!(runs("rs", "fn f<'a>(x: &'a u8) -> u32 { 42 }", NUMBER), vec!["42"]);
        assert!(runs("rs", "fn f<'a>(x: &'a u8)", STRING).is_empty(), "a lifetime is not a string");
    }

    #[test]
    fn shell() {
        let line = "if [ $# -gt 0 ]; then echo 'a b' # done";
        assert_eq!(runs("sh", line, KEYWORD), vec!["if", "then"]);
        assert_eq!(runs("sh", line, STRING), vec!["'a b'"]);
        assert_eq!(runs("sh", line, COMMENT), vec!["# done"]);
    }

    #[test]
    fn python() {
        let line = "def f(x): return x + 1.5  # sum";
        assert_eq!(runs("py", line, KEYWORD), vec!["def", "return"]);
        assert_eq!(runs("py", line, NUMBER), vec!["1.5"]);
        assert_eq!(runs("py", line, COMMENT), vec!["# sum"]);
        // A docstring is a string on its first line and plain after.
        assert_eq!(runs("py", "\"\"\"Doc", STRING).concat(), "\"\"\"Doc");
        assert!(runs("py", "more words", STRING).is_empty());
    }

    #[test]
    fn script() {
        let line = "const url = `http://x//y`; // why";
        assert_eq!(runs("ts", line, KEYWORD), vec!["const"]);
        assert_eq!(runs("ts", line, STRING), vec!["`http://x//y`"]);
        assert_eq!(runs("js", line, COMMENT), vec!["// why"]);
    }

    #[test]
    fn toml_and_yaml() {
        assert_eq!(runs("toml", "on = true # yes", KEYWORD), vec!["true"]);
        assert_eq!(runs("toml", "port = 8080", NUMBER), vec!["8080"]);
        assert_eq!(runs("yaml", "name: \"imp\" # tag", STRING), vec!["\"imp\""]);
        assert_eq!(runs("yml", "x: null", KEYWORD), vec!["null"]);
    }

    #[test]
    fn unknown_extensions_stay_plain() {
        assert!(kinds("txt", "let x = 1 # no").iter().all(|k| *k == 0));
    }
}
