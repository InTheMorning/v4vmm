# Column Text Truncation

## Purpose

**ADR 0063 owns this rule.** This document explains the defect, records what
was tried, and names what is still unknown. It states no rule of its own, and
a reader who needs the binding form reads the ADR.

Stop text from rendering as `...` with no words. Record what is known, what was
tried, and what stays unknown, so the person who reads this does not do the same
three attempts again.

## The Defect

Text stacked in a flex column, with `truncate()` on the text element, renders
the ellipsis and drops every word. The element keeps its space in the layout.
The text is in the view model, and it reaches the element.

It shipped on 2026-09-08 in the `Show` dashboard. Every card summary line and
every panel value read `...`. An operator reported that this has happened more
than once in this repository.

## How To Recognise It

Two text elements in the same container, one readable and one not:

```text
Host
...
Local - Reachable
```

`Host` is the label, `Local - Reachable` is the detail, and the middle line is
the value. All three sit in the same flex column and come from the same view
model. The only difference is `truncate()` on the middle one.

**That is the signal.** When neighbouring text renders and one element does not,
look at what is different about that element, not at the container. Two fix
attempts went to the container and both failed.

## Prohibited Fix

Do not call `truncate()` on text stacked in a flex column.

Do not try to repair it with a width. Both of these were tried on 2026-09-08 and
neither changed the render:

- `w_full()` on the truncating element
- moving `min_w_0()` from the truncating element to the parent column

## Required Mitigation

Use `overflow_hidden()` for column text. The text clips at the edge and stays
readable. There is no ellipsis, which is a known cost of this mitigation.

`truncate()` is correct where the element has a definite width:

- a row item that flexes, with `flex_1()`. See
  `src/ui/shells/queue_now_playing.rs`, which truncates track titles correctly.
- an element with an explicit bound, `max_w()` or `w()`. See the state badge in
  `src/ui/composites/show_card.rs`.

## Current Guard

`tests/architecture_tests.rs`,
`adr_0063_column_text_does_not_truncate`.

It covers `src/ui/composites/show_card.rs` and
`src/ui/composites/show_detail_panel.rs`. It permits `truncate()` on an element
whose chain holds `flex_1()`, `max_w(`, or `.w(`.

The guard is narrow on purpose. A scan on 2026-09-08 found 32 `truncate()` sites
under `src/ui/`, and 27 carried no width constraint, but most of those render
correctly because they sit in rows. A repository-wide rule would fail on correct
code, and a wrong guard blocks work.

## What Is Still Unknown

**The root cause.** No agent can run this app, because GPUI does not start an
X11 client in an agent session. Thus no agent looked at the render directly.
The three attempts came from layout rules, not from evidence, and the first two
were wrong.

Open questions for whoever can attach a running app:

- Does `truncate()` resolve the element width to zero in a column, or does it
  fail to lay out the text run at all?
- Does the parent chain of `min_w_0` and `overflow_hidden` on the card change
  that result?
- Does the same defect appear in a plain `div` column outside this surface, or
  does it need the card grid?
- Is a fix available in the renderer, so `truncate()` becomes safe in a column
  and the ellipsis returns?

Answer the first question, and a real fix can probably replace the mitigation.
Until then the mitigation stands, and the guard holds the line.

## The Wider Lesson

A wrong guard is worse than no guard. The first version of this guard
**required** `w_full()` on truncating elements, which is the pattern that does
not work. It would have enforced the defect.

When a rule comes from a theory that nobody has seen in operation, write the
guard after the fix is confirmed, not with it.

## References

- `AGENTS.md`, the conventions section
- ADR 0063, `docs/adr/0063-show-dashboard-layout.md`
- `docs/pending-human-checks.md`, for the checks that need a person
