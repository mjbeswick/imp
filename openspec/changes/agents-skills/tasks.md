# Tasks

- [x] 1.1 Load `.agents/skills`. Add it to the skill sources, skip non-directory entries, and order sources global, `.imp`, `.agents`, extra so the most specific wins. Verify: a skill in `.agents` appears, `.openspec-target` is ignored, and a duplicate name resolves to `.agents` over `.imp` and to extra over both.
- [x] 1.2 Load `.agents/agents`. Add it to `agent_dirs` before `.imp`, keep `.md` filtering, and let the first source win. Verify: `.agents/agents/reviewer.md` is listed, a duplicate in `.imp` loses, and `README.txt` is ignored.
- [x] 1.3 Extend `/agent`. No argument keeps the picker, `/agent <name>` sends the switch, `/agent current` asks for a report, and an unknown name lists the roster. Verify: command tests see `SetAgent` and `ShowAgent`, and `describe_agent` names model, thinking, and tools.
- [x] 1.4 Wire line mode and docs. Extend the line-mode arm with a direct name and `current`, and document `.agents` plus the precedence in the README. Verify: `/agent reviewer` switches, and the README names both `.agents/skills` and `.agents/agents`.
- [x] 1.5 Run the checks. Verify: `cargo test`, `openspec validate --all --strict`, and `./scripts/size.sh` pass.
