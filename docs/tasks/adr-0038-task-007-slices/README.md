# ADR 0038 Task 007 — Slice Plans

Each slice is an executable brief for a sonnet-class subagent. Read
`../adr-0038-task-007-screen-decomposition.md` first for the overall
plan and surface inventory.

## Execution Order

```
00 → L1 → L2 → L3 → L4 → L5 → L6 ─┐
                                  ├─→ F
00 → D1 → D2 → D3 → D4 → D5 → D6 ─┘
```

Library (L*) and Discover (D*) chains are independent after Slice 0.
They can run in parallel between two subagents but each chain must
run sequentially. Slice F runs after all 12 surface slices land.

## Subagent Invocation Template

For each slice, dispatch one subagent. Use `subagent_type: Explore`
won't work — the subagent must write code. Use the general-purpose
agent or a code-focused agent type.

```
Agent({
    description: "Slice <ID> screen decomposition",
    subagent_type: "general-purpose",
    model: "sonnet",
    prompt: """
You are executing Slice <ID> of ADR 0038 Task 007 (screen
decomposition).

Read this file first and follow it exactly:
docs/tasks/adr-0038-task-007-slices/<NN>-<slice-name>.md

Background context (do not re-derive):
- ADR 0038 is the presentation contract enforcement project.
- Task 007 splits src/library.rs and src/search.rs into per-surface
  shell modules under src/ui/shells/{library,discover}/.
- Task 006 is already complete; PageVm helpers exist for every
  entity detail surface.

Key constraints:
- One surface per commit. Compile and test green at every commit.
- No behavior changes. Visual output must match pre-slice exactly.
- Mutators stay on the screen struct (LibraryApp / SearchApp).
  Surfaces invoke them via cx.listener(...) callbacks.
- Do NOT extract shared helpers across Library/Discover during
  this task. Different signatures would force premature unification.
- If a structural problem emerges (unclear boundary, helper used by
  unexpected surface, listener wiring fails to compile cleanly),
  STOP and report the obstacle. Do not improvise.

Verification before commit (all four MUST pass):
    cargo fmt -- --check
    cargo clippy --lib -- -D warnings
    cargo test --lib
    cargo test --tests

Use the commit message template in the slice plan. Use HEREDOC
syntax for the commit message body.

Report back with: the commit SHA, the resulting wc -l for affected
files, and any deviations from the slice plan.
"""
})
```

## Per-Slice Caveats

- **Slice 0** is trivial (module wiring only). A haiku-class subagent
  can handle it; sonnet is overkill but harmless.
- **Slices L2 (feed_list), L5/L6 and D5/D6 (track core/metadata
  splits)** have unclear render boundaries. The slice plans say
  "stop and report" if the boundary isn't obvious — honor that.
- **Slice L4 (feed_detail)** is the largest planned single-surface
  move (~450 LOC). May exceed 500 LOC after the move; the plan says
  to pause for review if so.
- **Slice F** must wait for ALL 12 surfaces to land. Running it early
  fails the file-existence guards.

## When To Run Each Slice

Suggested flow:

1. Run Slice 0 (any subagent) — module structure.
2. Pick L1 + D1 to validate the pattern (smallest in each chain) —
   verify the listener wiring approach works in real code before
   committing to the rest.
3. Run remaining L* sequentially as one chain.
4. Run remaining D* sequentially as a second chain (parallel with L*
   if desired).
5. After all 12 land, run Slice F.

Total commits: 14 (1 module setup + 12 surfaces + 1 final).
