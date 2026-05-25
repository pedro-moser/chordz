# Qwen / ohmypi Harness Guide

## Purpose

This repository is intentionally being shaped so a local Qwen model through ohmypi can make progress without drifting into loops or over-broad implementation. The model should receive small tasks, stable context, and executable acceptance criteria.

The canonical project context is:

- [ARCHITECTURE.md](./ARCHITECTURE.md)
- [VOICING_ENGINE.md](./VOICING_ENGINE.md)
- [TASKS.md](./TASKS.md)

Do not use session logs as the source of truth. Logs can explain history, but the docs above define current intent.

## Model Operating Rules

When assigning a task to Qwen:

1. Give one narrow objective.
2. Name the files it may edit.
3. State the tests or commands that must pass.
4. Include at least one musical acceptance example.
5. Tell it to stop after the objective is complete.

Avoid prompts like:

- "Continue the plan."
- "Implement the voicing engine."
- "Make the TUI."
- "Do the next phase."

Prefer prompts like:

- "Add `VoicingRecipe::Shell` and tests proving a G7 shell can omit the fifth."
- "Create a pure ASCII renderer for one `Fingering`; do not touch UI/event code."
- "Change `root_to_pc` to return `Option<u8>` and update callers/tests."

## Required Work Loop

Every implementation turn should follow this loop:

1. Read the relevant spec section.
2. Inspect existing code in the files to be edited.
3. Write a short implementation plan.
4. Make the smallest coherent change.
5. Run `cargo test --all-targets`.
6. Report changed files, tests, and any remaining caveats.

If a task touches musical behavior, add or update tests before claiming completion.

## Phase-End Review Gate

At the end of each milestone or phase, run a separate review pass before calling the phase complete.

Required sequence:

1. Run `cargo test --all-targets`.
2. Inspect `git diff --stat` and `git diff`.
3. Ask OMP's `reviewer` agent to review the patch.
4. Fix any confirmed correctness issues.
5. Re-run `cargo test --all-targets`.
6. Only then summarize the phase as complete.

Review prompt template:

```text
Review the current chordz patch as a phase-end reviewer.

Canonical docs:
- docs/ARCHITECTURE.md
- docs/VOICING_ENGINE.md
- docs/TASKS.md

Review scope:
- correctness bugs
- musical-domain regressions
- mismatch with the canonical specs
- missing tests for changed behavior

Do not edit files. Report only actionable findings anchored to the diff.
```

The existing OMP `reviewer` agent is acceptable for this gate. If OMP later has a Codex/OpenAI-backed reviewer model, use that for the phase-end review and keep Qwen as the implementation worker.

## Stop Conditions

The model should stop and ask for direction when:

- The task requires choosing between conflicting musical interpretations.
- It needs to change public data structures across several modules.
- It cannot produce a deterministic test for the behavior.
- The existing code contradicts the canonical docs.

The model should not stop merely because implementation is multi-step. Instead, it should reduce the task to the smallest slice that satisfies the given acceptance criteria.

## Guardrails Against Known Failure Modes

### Looping on Plan Edits

Plan/spec edits should be plain file edits in `docs/`. If a tool fails, inspect the file and use a simpler patch. Do not repeatedly retry the same failed edit.

### Overclaiming Phase Completion

Do not say "phase complete" unless:

- The requested files are implemented.
- Tests pass.
- The final behavior can be observed or asserted.
- The implementation matches the musical examples in the task.

### Collapsing Jazz Voicings Into Full Chord Stacks

For extended chords, do not require every interval to appear in one fingering. Modern jazz guitar voicings often omit roots/fifths and emphasize guide tones plus color tones.

### Ignoring the Binary

Library tests are not enough forever. Once render/UI work begins, keep a small `cargo run` smoke path that shows something real.

## Suggested ohmypi Prompt Template

```text
You are working in /home/pedro/Projects/chordz.

Canonical docs:
- docs/ARCHITECTURE.md
- docs/VOICING_ENGINE.md
- docs/TASKS.md

Task:
<one narrow objective>

Allowed files:
<explicit list>

Acceptance:
- <musical behavior or UI behavior>
- <test behavior>
- `cargo test --all-targets` passes

Stop after completing this task. Do not start the next task.
```

## Current Best Next Task

Use [TASKS.md](./TASKS.md). At the time this guide was written, the best next engineering move is to stabilize the domain model before building UI: introduce recipe/voice-set concepts and tests for rootless/shell behavior.
