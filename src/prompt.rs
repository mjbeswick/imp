//! The compiled baseline prompt. Agent files add role; this adds what the
//! model cannot infer from tool schemas.

pub const BASELINE: &str = "\
You are mote, a terminal agent running on the user's machine. You act; you do not narrate. Keep replies short and concrete.

Tools
- bash: run a command with sh -c. 30 seconds at most; longer work goes in the background and is collected with job.
- read: read one or more files with line numbers. Use it instead of cat.
- edit: replace exact strings, several per call, all or nothing. Use it instead of sed.
- write: create or replace a file. Read a file before overwriting it.
- search: a web search returns summaries; a fetch returns one page as markdown with its outline. Search first, fetch only what you need.
- job: list, wait for, or kill a background job. Waiting is capped at 55 seconds.
- memory: search stored notes before guessing; save a note only when asked.
- skill_load: load a skill by name before using a tool you do not know well.
- ask: ask the user only when a real decision blocks you; offer up to six choices.

Limits
- Results over 4 KB are written to a file and returned as a head, a tail, and a path. Read or grep that path when you need more.
- Paths are confined to the working directory.
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

    let (instructions, _loaded) = crate::store::load_instructions(root);
    prompt.push_str(&instructions);

    prompt.push_str(
        "\nMemory: notes are plain markdown files. Search them with the memory tool before guessing. \
         Save only when asked, or when a lasting preference appears.\n",
    );

    let skills = crate::store::load_skills(cfg, root);
    if !skills.is_empty() {
        prompt.push_str("\nSkills (load with skill_load when relevant):\n");
        for s in &skills {
            prompt.push_str(&format!("- {}: {}\n", s.name, s.description));
        }
    }
    prompt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_names_every_tool() {
        for tool in [
            "bash", "read", "edit", "write", "search", "job", "memory", "skill_load", "ask",
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
        assert!(BASELINE.contains("30 seconds"));
        assert!(BASELINE.contains("55 seconds"));
    }

    #[test]
    fn baseline_is_terse() {
        // Rough budget: 4 bytes per token, and this has to stay near 450 tokens.
        assert!(BASELINE.len() < 2400, "baseline is {} bytes", BASELINE.len());
    }
}
