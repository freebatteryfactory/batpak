# CLAUDE.md

Claude Code auto-loads this file. The authoritative, tool-agnostic operating
doctrine for this repository lives in **`AGENTS.md`** (shared with Codex and other
agent tools). It is imported below so Claude loads exactly the same rules — there
is deliberately no second source of truth to drift.

Read it before writing a line: the zero-`#[allow]` tripwire, the
`panic!`/`.unwrap()`-denied-everywhere / `.expect()`-in-tests discipline, the
`batpak < syncbat < netbat` layer boundary, the sync-first/`flume` concurrency
rule, the canonical `just` command surface, and the mutation/structural gates are
all non-negotiable and all enforced.

@AGENTS.md
