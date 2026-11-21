//! The compiled baseline prompt. Agent files add role; this adds what the
//! model cannot infer from tool schemas.

pub const BASELINE: &str = "\
You are imp, a terminal agent running on the user's machine. You act; you do not narrate. Keep replies short and concrete.

Tools
- bash: run a command with sh -c, 30 seconds at most; longer work goes in the background. show puts coloured output, e.g. chafa --format symbols, on screen.
- read: files with line numbers, or list a directory with depth; ask for every path in one call.
- edit: replace exact strings, several per call, all or nothing. Use it instead of sed.
- write: create or replace a file. Read a file before overwriting it.
- search: a web search returns summaries with URLs. Read a result with fetch.
- fetch: one HTTP request from this machine, the local network included; GET by default, or POST, PUT, PATCH, DELETE with a body. HTML comes back as markdown, feeds as a list, JSON and text as they are; results are capped. imp adds configured credentials for their host. Use it instead of curl; curl only for streams and uploads.
- job: list with progress, wait (55 seconds at most), kill, or check: after_s later you are woken to report on long work, such as an rsync --info=progress2 copy.
- memory: search stored notes before guessing; save a note only when asked.
- skill_load: load a skill by name before using a tool you do not know well.
- /reload re-reads instructions, skills, and memory. Never tell the user to restart for them.
- ask: ask the user only when a real decision blocks you; offer up to six choices.

Limits
- Results over 4 KB go to a buffer: you get a head, a tail, and imp://out/<n>. Read that address with offset and limit when you need more.
- read, edit, and write reach only the session directory. bash is not jailed, so keep commands inside the project unless the user asks otherwise.
- Reply in markdown: tables render as boxed tables. Content width arrives as an environment line when it changes; keep code, tables, and rules inside it, and prefer lists over tables when narrow.
- Never run interactive commands. stdin is closed.
- Confirmations belong to the user. If something is refused, say so and continue.

Style
- Lead with the result. Do not restate the question or narrate what you are about to do.
- One or two sentences unless detail was asked for. No headings or bullet lists for short answers.
- Never invent file contents, paths, or command output.
- For a multi-step task, do the steps in order and report only what changed.
";

pub fn compose(root: &std::path::Path, cfg: &crate::config::Config) -> String {
    let mut prompt = BASELINE.to_string();
    if !cfg.provider.caps().search {
        prompt = prompt
            .lines()
            .filter(|line| !line.starts_with("- search:"))
            .collect::<Vec<_>>()
            .join("\n");
        prompt.push('\n');
    }

    // The date itself arrives as a system note before each day's first request.
    prompt.push_str(
        "\nA system note gives today's date. For anything that changes, such as news, prices, weather, \
         results, or releases, search, then fetch the best source, and say how current the answer is.\n",
    );

    if let Some(dir) = crate::tools::scratch() {
        prompt.push_str(&format!(
            "Scratch: {} is yours, as $IMP_TMP in commands. Put downloads and temporary files there, never /tmp; \
             it needs no confirmation and is emptied when imp exits.\n",
            dir.display()
        ));
    }

    let (instructions, _loaded) = crate::store::load_instructions(root);
    prompt.push_str(&instructions);

    prompt.push_str(
        "\nMemory: notes are plain markdown files. Search them with the memory tool before guessing. \
         Save only when asked, or when a lasting preference appears.\n",
    );
    let notes = crate::store::Memory::discover(cfg, root).index(40);
    if !notes.is_empty() {
        prompt.push_str("Notes (search for the body when one applies):\n");
        for (slug, title) in &notes {
            prompt.push_str(&format!("- {slug}: {title}\n"));
        }
    }

    let skills = crate::store::load_skills(cfg, root);
    if !skills.is_empty() {
        prompt.push_str("\nSkills (load with skill_load when relevant):\n");
        for s in &skills {
            prompt.push_str(&format!("- {}: {}\n", s.name, s.description));
        }
    }
    prompt
}

/// Today as days since 1970-01-01, in UTC.
// ponytail: UTC, so up to a day off near midnight far from Greenwich; localtime_r if that matters.
pub fn day_number() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
        / 86_400
}

/// A day count since 1970-01-01 as a weekday and civil date, such as
/// `Saturday 21 November 2026`.
pub fn date_of(days: u64) -> String {
    // Howard Hinnant's civil-from-days, for days since the epoch.
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    const MONTHS: [&str; 12] = [
        "January", "February", "March", "April", "May", "June", "July", "August", "September", "October",
        "November", "December",
    ];
    const DAYS: [&str; 7] = ["Thursday", "Friday", "Saturday", "Sunday", "Monday", "Tuesday", "Wednesday"];
    format!("{} {day} {} {year}", DAYS[(days % 7) as usize], MONTHS[(month - 1) as usize])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_are_civil() {
        assert_eq!(date_of(0), "Thursday 1 January 1970");
        assert_eq!(date_of(20_413), "Friday 21 November 2025");
        assert_eq!(date_of(11_016), "Tuesday 29 February 2000");
        assert_eq!(date_of(20_778), "Saturday 21 November 2026");
    }

    #[test]
    fn the_prompt_carries_today_and_the_fresh_data_rule() {
        let d = std::env::temp_dir().join(format!("imp-prompt-date-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&d);
        let text = compose(&d, &crate::config::Config::default());
        assert!(text.contains("A system note gives today's date."), "{text}");
        assert!(text.contains("search, then fetch the best source"));
    }

    #[test]
    fn the_prompt_names_reload() {
        assert!(BASELINE.contains("/reload"), "the model may still say restart");
    }

    #[test]
    fn baseline_names_every_tool() {
        for tool in [
            "bash", "read", "edit", "write", "search", "fetch", "job", "memory", "skill_load", "ask",
        ] {
            assert!(
                BASELINE.contains(tool),
                "baseline prompt lost the {tool} tool"
            );
        }
    }

    #[test]
    fn baseline_states_the_caps() {
        assert!(BASELINE.contains("4 KB"));
        assert!(BASELINE.contains("list a directory"));
        assert!(BASELINE.contains("imp://out/"));
        assert!(BASELINE.contains("bash is not jailed"));
        assert!(!BASELINE.contains("working directory"));
        assert!(!BASELINE.contains("spill"));
        assert!(BASELINE.contains("30 seconds"));
        assert!(BASELINE.contains("55 seconds"));
    }

    #[test]
    fn a_provider_without_search_has_no_search_line() {
        let root = std::env::temp_dir();
        let text = compose(&root, &crate::config::Config::default());
        assert!(text.contains("- search:"), "deepseek lost its search line");

        let mut cfg = crate::config::Config::default();
        cfg.set_provider(crate::providers::Provider::OpenAi);
        let text = compose(&root, &cfg);
        assert!(!text.contains("- search:"), "{text}");
        assert!(text.contains("- bash:"), "the other tools vanished");
    }

    #[test]
    fn skill_index_is_one_line_each() {
        let d = std::env::temp_dir().join(format!("imp-skills-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        for i in 0..45 {
            let dir = d.join(format!("s{i}"));
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join("SKILL.md"),
                format!("---\nname: s{i}\ndescription: does thing {i}\n---\nbody {i}\n"),
            )
            .unwrap();
        }
        let mut cfg = crate::config::Config::default();
        cfg.skills_dir = Some(d.clone());
        let root = std::env::temp_dir();
        let skills = crate::store::load_skills(&cfg, &root);
        assert_eq!(skills.len(), 40, "index must cap at 40");
        let text = compose(&root, &cfg);
        let tail = text
            .split("Skills (load with skill_load when relevant):")
            .nth(1)
            .unwrap_or("");
        let index: Vec<&str> = tail.lines().filter(|l| l.starts_with("- ")).collect();
        assert!(index.len() <= 40);
        assert!(index.iter().any(|l| l.contains("does thing 0")));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn notes_are_indexed_by_title_and_capped() {
        let d = std::env::temp_dir().join(format!("imp-notes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let mem = d.join(".imp").join("memory");
        std::fs::create_dir_all(&mem).unwrap();
        std::fs::write(
            mem.join("user-location-woking-uk.md"),
            "# User location: Woking, UK\n\nThe user lives in Woking.\n",
        )
        .unwrap();
        let text = compose(&d, &crate::config::Config::default());
        assert!(text.contains("- user-location-woking-uk: User location: Woking, UK"), "{text}");
        assert!(!text.contains("The user lives in Woking"), "a body leaked into the prompt");

        for i in 0..45 {
            std::fs::write(mem.join(format!("n{i:02}.md")), format!("# note {i}\n")).unwrap();
        }
        let m = crate::store::Memory { dirs: vec![mem.clone()] };
        assert_eq!(m.index(40).len(), 40);
        assert_eq!(m.index(40)[0].0, "n00", "not sorted");
        let hits = m.search("woking location", 5);
        assert_eq!(hits.len(), 1, "words apart were not found");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn baseline_prefers_fetch_over_curl() {
        assert!(BASELINE.contains("Use it instead of curl"), "no fetch rule");
        assert!(BASELINE.contains("curl"), "no curl limit");
    }

    #[test]
    fn baseline_names_the_width_rule() {
        assert!(
            BASELINE.contains("Content width"),
            "the prompt does not mention the width"
        );
    }

    #[test]
    fn baseline_is_terse() {
        // Rough budget: 4 bytes per token, and this has to stay near 450 tokens.
        assert!(BASELINE.len() < 2400, "baseline is {} bytes", BASELINE.len());
    }
}
