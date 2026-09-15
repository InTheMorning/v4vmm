# Upstream PR Preparation: Input Double-Click And Joined Emoji Selection

Status: Draft - 2026-09-15. Two native failures reproduced; candidate correction
verified in isolated 0.6.1 sources and the publication workspace. Prepared for a
future upstream PR. The operator published the correction in their fork; the
app now pins it under [ADR 0072](../adr/0072-pinned-gpui-base-selection-corrections.md).
No upstream issue or PR has been submitted. All focused X11 correction checks
are accepted on 2026-09-15; IME composition and Wayland remain untested.
[ADR 0071 task 001](../tasks/adr-0071-task-001-shared-text-selection.md)
records the native evidence, accepted final preservation and confirmed
fixture/scratch cleanup. The application packet is complete.

## Proposed PR Title And Summary

**input: Hit-test words by glyph and preserve complete grapheme selections**

Double-clicking a joined emoji at the end of an input can select nothing.
Adding a period after it makes a selection visible, but the selected text
contains only the first emoji scalar. These are separate defects: pointer
resolution can return the position after the final character, and word ranges
can end inside a grapheme cluster. A grapheme cluster is a sequence of code
points displayed as one character, such as `👩‍💻`.

The proposed correction uses glyph hit testing for double-click and expands
the existing character-class word ranges to grapheme boundaries. It keeps
ordinary caret movement on its existing path. The fix belongs in gpui-base;
it requires no change to GPUI's renderer or the downstream PRIMARY publisher.

## Native Reproduction And Exact Evidence

Observed in v4vmm toolbar search on Linux/X11, using xfce4-terminal as the
PRIMARY recipient. At reproduction, the application used published, exact-pinned gpui-component,
gpui-base and gpui-kit-assets 0.6.1, with gpui-pre/platform 0.3.1 and Rust 1.97.1.
The precise native font/fallback configuration was not captured. Wayland and
IME composition were not exercised in this report.

Paste each complete value with Ctrl+A/Ctrl+V into the input. Do not construct
the second sample by typing a period into the preceding selection, and do not
submit the search. Double-click the middle of the joined emoji.

| Case | Complete input value | Expected | Observed on X11 |
| --- | --- | --- | --- |
| A: final glyph | `中文 👩‍💻` | Select the complete `👩‍💻` | Nothing selected |
| B: following punctuation | `中文 👩‍💻.` | Select the complete `👩‍💻`, excluding `.` | Highlight covers the emoji, but PRIMARY contains only `👩` |

The operator first verified that the whole input retained these code points:

```text
U+4E2D U+6587 U+0020 U+1F469 U+200D U+1F4BB
```

For case B, prepare this command in a desktop shell without pressing Enter:

```bash
xclip -selection primary -out -target UTF8_STRING | python3 -c 'import sys; print(repr(sys.stdin.buffer.read().decode("utf-8")))'
```

Return to the input, press Right to collapse any selection, then double-click
the emoji. Do not Select All or Copy. Return to the shell and execute the
prepared command. Preparing it first avoids replacing the app's PRIMARY
selection by selecting the command afterward.

```text
Expected: '👩\u200d💻'
Observed: '👩'
```

The expected emoji consists of U+1F469, U+200D and U+1F4BB. The observed payload
contains only U+1F469. In the full sample the emoji occupies UTF-8 bytes
`7..18`; the upstream word calculation returns `7..11`. The latter range is
established by source/tests; the operator measured the payload, not an internal
range. A correct-looking highlight is therefore insufficient acceptance.

The input observer in v4vmm publishes `InputBaseState::selected_text()` unchanged.
The standalone upstream regressions below reproduce the bad range without
that observer or a native clipboard. The observed PRIMARY failure is not a
transport truncation. Native Ctrl+C on the unpatched sample was not separately
verified; the candidate's exact Copy behavior is covered by its mock test.

## Causes And Shared Owners

**A — caret hit testing used for a word click.**
[`InputBaseState::on_mouse_down`](https://github.com/longbridge/gpui-kit/blob/501c73923280859a5de2b16fe64d4aac960bb040/crates/base/src/input/base/state.rs#L2214)
resolves a caret position before calling `select_word`. For a single-line
input this reaches `LineLayout::closest_index_for_x`. After the final glyph's
start, that function returns the line's UTF-8 length unless the entire line
has byte length one. The word selector finds no character at that end offset
and leaves no word selected.

The deterministic regression supplies a shaped line for `中文 👩‍💻`: the emoji
begins at x=30, ends at x=50 and starts at byte 7. Clicking x=31 resolves to
byte 18 before the fix. The test supplies this geometry directly; it does not
depend on an installed emoji font. The candidate tests x=31, 35, 40 and 49,
with and without the trailing period, in both InputState and TextareaState.

**B — scalar word boundaries split a displayed character.**
[`TextSelector::word_range`](https://github.com/longbridge/gpui-kit/blob/501c73923280859a5de2b16fe64d4aac960bb040/crates/base/src/input/base/selection.rs#L79)
uses [`word_range_from_chars`](https://github.com/longbridge/gpui-kit/blob/501c73923280859a5de2b16fe64d4aac960bb040/crates/base/src/text_boundary.rs#L28).
The woman scalar has character class `Other`; its range ends after that scalar,
before the ZWJ and laptop. Selection geometry can cover the shaped glyph even
though the stored range stops inside it. Fixing the pointer offset alone still
leaves this partial selection; expanding boundaries alone cannot fix case A's
missing character at the end offset.

## Upstream Status At Investigation

Verified on 2026-09-14; refresh these facts before submission.

- The crate index listed gpui-base 0.6.1 as the newest published version;
  GitHub's [latest release](https://github.com/longbridge/gpui-kit/releases/tag/v0.6.1)
  was also v0.6.1.
- The published gpui-base crate's `.cargo_vcs_info.json` identifies commit
  `96103905ea0c9c199db206ada7a9b2e3114e6339`. This is the source baseline for
  the attached patches; do not substitute the differently named release-tag
  commit without checking the patch.
- Remote main was `501c73923280859a5de2b16fe64d4aac960bb040`. Its `selection.rs`,
  `text_boundary.rs` and `text_wrapper.rs` were byte-identical to the published
  0.6.1 files. Its input mouse handler and position resolver were unchanged.
- That main revision's [workspace manifest](https://github.com/longbridge/gpui-kit/blob/501c73923280859a5de2b16fe64d4aac960bb040/Cargo.toml#L60)
  uses gpui-pre 0.3.5. Inspection of its
  [published source archive](https://static.crates.io/crates/gpui-pre/gpui-pre-0.3.5.crate)
  found `src/text_system/line_layout.rs` byte-identical to 0.3.1.

Both defects remain in those sources. This comparison is not a native test or
a full workspace build of upstream main. Existing upstream issues and unmerged
PRs have not been exhaustively searched; check for duplicates before submitting.

## Review Artifacts And Scope

Apply the artifacts in this order:

1. [Regression tests](adr-0071-gpui-base-emoji-selection-tests.patch): three tests,
   135 added lines across three upstream files; no production changes.
2. [Candidate correction](adr-0071-gpui-base-emoji-selection.patch): 55 added and
   13 removed lines across the same files.

The implementation changes are limited to:

- `crates/base/src/input/base/state.rs`: use glyph hit testing for double-click;
  retain the existing caret resolver for other pointer gestures.
- `crates/base/src/text_boundary.rs`: expand word endpoints with the existing
  unicode-segmentation dependency's `GraphemeCursor`.
- `crates/base/src/input/base/selection.rs`: delegate rope word selection to
  that shared boundary owner for the current logical line, including its
  terminator. Borrow contiguous text; copy a line only when it spans chunks.

There are no dependency additions, GPUI source changes, PRIMARY integration
changes, application handlers or layout tokens in these patches.

## Regression Evidence And Replay

The test-only patch was replayed against pristine published 0.6.1 sources on
2026-09-14. All three new tests failed with the same payload differences:

| Test | Unpatched result | Expected |
| --- | --- | --- |
| `input::state::tests::word_selection_hits_complete_final_emoji` | Empty selection | `👩‍💻` |
| `input::selection::tests::word_selection_preserves_graphemes_in_rope` | `👩` | `👩‍💻` |
| `text_boundary::tests::word_selection_preserves_graphemes` | `👩` | `👩‍💻` |

The filter `word_selection_` also matches one existing passing test. Thus the
expected baseline summary is **1 passed, 3 failed**. After applying the
correction, the replay returned **4 passed, 0 failed**. The two patches together
reconstruct the previously verified candidate files byte-for-byte.

The recorded full candidate verification was Green:

- gpui-base: 900 unit tests, changed-file formatting and strict Clippy.
- v4vmm with an isolated command-line dependency override: check, formatting,
  strict Clippy, 1,390 unit tests and 247 architecture guards. Ten existing
  ignored doctests remained ignored. Local socket access was required by the
  app's service fixtures.

These runs used the published crate's normalized manifest in an isolated copy,
with its existing dependencies and a crate README supplied for a workspace-relative
Markdown test fixture. They did not build the full upstream main workspace or
run a native application. The summaries above preserve the useful evidence;
the original `/tmp` build logs are not required to understand either defect.

During fork preparation on 2026-09-14, both patches also applied cleanly to a
Git checkout of the publication commit. The original upstream workspace then
passed all four focused tests, all 900 gpui-base unit tests, strict Clippy for
the base library/tests, and workspace formatting using its own unchanged
lockfile. That lockfile resolves gpui-pre/platform 0.3.2; the isolated
app checks above retain the application's 0.3.1 pins. This additional run is
publication-commit evidence, not a build of current upstream main or a native
acceptance result.

To prepare a future upstream checkout, start in the v4vmm repository root:

```bash
selection_pr_patches="$PWD/docs/reviews"
selection_pr_checkout=$(mktemp -d /tmp/gpui-selection-pr-XXXXXX)
git clone https://github.com/longbridge/gpui-kit.git "$selection_pr_checkout"
git -C "$selection_pr_checkout" checkout --detach 96103905ea0c9c199db206ada7a9b2e3114e6339
git -C "$selection_pr_checkout" apply --check "$selection_pr_patches/adr-0071-gpui-base-emoji-selection-tests.patch"
git -C "$selection_pr_checkout" apply "$selection_pr_patches/adr-0071-gpui-base-emoji-selection-tests.patch"
cargo +1.97.1 test --manifest-path "$selection_pr_checkout/Cargo.toml" --locked -p gpui-base --lib word_selection_
```

Expect the three failures above. Then apply the implementation and repeat:

```bash
git -C "$selection_pr_checkout" apply --check "$selection_pr_patches/adr-0071-gpui-base-emoji-selection.patch"
git -C "$selection_pr_checkout" apply "$selection_pr_patches/adr-0071-gpui-base-emoji-selection.patch"
cargo +1.97.1 test --manifest-path "$selection_pr_checkout/Cargo.toml" --locked -p gpui-base --lib word_selection_
cargo +1.97.1 test --manifest-path "$selection_pr_checkout/Cargo.toml" --locked -p gpui-base --lib
cargo +1.97.1 clippy --manifest-path "$selection_pr_checkout/Cargo.toml" --locked -p gpui-base --lib --tests -- -D warnings
cargo +1.97.1 fmt --manifest-path "$selection_pr_checkout/Cargo.toml" --all -- --check
```

The focused and full base suites have now passed in the original publication
workspace. Before submission, reproduce the baseline failures there as well,
then rebase onto the then-current upstream main and repeat checks. Report any
new toolchain or dependency requirement explicitly. Keep the checkout if preparing commits;
otherwise print and inspect the disposable checkout path before removing it:

```bash
printf '%s\n' "$selection_pr_checkout"
```

After confirming it is the `/tmp/gpui-selection-pr-*` directory created above:

```bash
rm -rf -- "$selection_pr_checkout"
unset selection_pr_checkout selection_pr_patches
```

## Remaining Review And Acceptance

### Published Fork Commits

The following commits contain exactly the two patches, based on publication
commit `96103905ea0c9c199db206ada7a9b2e3114e6339`:

- Tests: `a131f4e0d15eda4cb892b12999c77ee1e56fa903`.
- Correction: `5463fe4e72fd740b0db08da92003488b32661867`.

They are attributed to `Codex <codex@localhost>` and published in
[InTheMorning/gpui-kit](https://github.com/InTheMorning/gpui-kit/tree/5463fe4e72fd740b0db08da92003488b32661867).
No commits were made in the v4vmm repository. The source Cargo fetched from
the remote matches the fully tested isolated candidate byte-for-byte.

The operator published branch `fix/input-emoji-selection` from the prepared
checkout with plain Git/SSH. Read-only verification on 2026-09-14 returned
`5463fe4e72fd740b0db08da92003488b32661867` for this command:

```bash
git ls-remote --exit-code https://github.com/InTheMorning/gpui-kit.git refs/heads/fix/input-emoji-selection
```

The app now pins that revision through a gpui-base-only Cargo override.
`Cargo.lock` changes only that package's source and registry checksum; all
versions and other packages are unchanged. App verification is recorded in
[task 001](../tasks/adr-0071-task-001-shared-text-selection.md#fork-integration--2026-09-14).
The temporary checkouts, source copies and
`target/adr-0071-gpui-base-selection.bundle` were removed during acceptance
cleanup on 2026-09-15. Before removal, both tracked patches were replayed and
verified to reconstruct their respective published commits byte-for-byte.
The tracked patches and published fork remain the durable reproduction source.

### Remaining Coverage

The native retries on the pinned correction, on 2026-09-14, returned
`'👩\u200d💻'` for both case A (`中文 👩‍💻`) and case B (`中文 👩‍💻.`).
Both exact PRIMARY checks are accepted: all three code points are retained
and the following period is excluded. The operator also confirmed complete
emoji highlighting in both cases with the period unhighlighted. Both toolbar
cases are accepted. Both cases also passed in Settings → Library's MusicIndex
endpoint, including exact PRIMARY, complete highlighting excluding the period,
and one Undo restoring the original endpoint after each sample. Both cases
also passed in the configuration editor's `musicindex_endpoint` draft,
including complete highlighting, exact PRIMARY excluding the period and one
Undo restoring the previous value after each sample. The remaining focused
checks were accepted on 2026-09-15 as recorded below.

On 2026-09-15 the configuration editor also passed the Unicode path word
regression: both accent forms (`café`, `café`), `music_dir`, each CJK character
(`中`, `文`) and `👩‍💻` retained exact highlighting and PRIMARY text. Full-path
triple-click also passed: complete highlighting and an exact Unicode comparison
confirmed the full path, including both accent forms and the emoji joiner.
One Undo restored the preceding value.

Native clipboard separation also passed in the editor: after double-clicking
the emoji without Copy, terminal middle-click pasted the complete emoji while
explicit Ctrl+Shift+V retained `CLIPBOARD-KEEP`. Explicit Copy also passed:
Ctrl+C on the selected editor emoji and Ctrl+Shift+V in the cleared terminal
scratch reader pasted the complete emoji. Replacing the selected emoji with
`.` also passed: the value became `中文 .`, one Undo restored `中文 👩‍💻`, Redo
restored the period, and another Undo restored the emoji. Each step preserved
the preceding text. Escape/Close/Reopen also passed: Escape moved focus to
Close editor while leaving the editor open; Enter closed it once without
reopening on key release. Reopen editor retained `musicindex_endpoint` and
the exact unsaved draft `中文 👩‍💻`. Incoming PRIMARY insertion also passed:
selecting the complete terminal emoji without Copy and middle-clicking after
the editor's existing emoji produced `中文 👩‍💻👩‍💻`. Undo, Redo and Undo
removed, restored and removed the complete inserted emoji while preserving
the preceding text. Reload file (discard draft) subsequently restored the
original editor endpoint; the editor was closed without saving and the terminal
scratch reader was stopped. In Settings → Library's MusicIndex endpoint, the
first, precomposed `café` in the Unicode path passed exact word highlighting
without slashes and outgoing PRIMARY (`'café'`). The second, decomposed `café`
also passed complete word/accent highlighting without slashes and exact PRIMARY
comparison with `cafe\u0301`. `music_dir` also passed whole-word highlighting,
including the underscore without slashes, and exact PRIMARY comparison.
Both `中` and `文` passed individual-character highlighting and exact PRIMARY
(`'中'` and `'文'`, respectively). The path's joined emoji also passed complete
highlighting without the preceding slash or `.flac` and exact PRIMARY
(`'👩\u200d💻'`). Settings full-value triple-click also passed complete
highlighting and exact PRIMARY comparison, preserving both accent forms,
the Chinese characters and the joined emoji. One Undo restored the original
endpoint without saving. Focused Settings correction checks are accepted.
The Producer Logs final-emoji check also passed: double-clicking `👩‍💻` on
`Final emoji: 中文 👩‍💻` highlighted the complete emoji without the preceding
space and returned exact PRIMARY `'👩\u200d💻'`. Explicit log Copy also passed:
Ctrl+C on the selected final emoji and inspection of the separate clipboard
returned exactly `'👩\u200d💻'`. Both accented words on the log's `Unicode path:`
line also passed complete word/accent highlighting without slashes and exact
PRIMARY: `'caf\xe9'` for the first and `'cafe\u0301'` for the second, preserving
their distinct accent forms. `music_dir`, `中` and `文` also passed individual
item highlighting and exact PRIMARY (`'music_dir'`, `'\u4e2d'` and `'\u6587'`,
respectively). Whole-line triple-click also passed: the `Unicode path:` label
and complete path were selected without the next line, and exact PRIMARY
comparison returned `PASS`, preserving both accent forms, the Chinese characters
and joined emoji without an LF terminator. The log read-only check also passed:
typing `X`, Backspace, Delete and Ctrl+V with the path line focused and selected
left the text intact and the app responsive. All focused X11 correction checks
are accepted. Final normal-fixture inspection on 2026-09-15 also passed: only
normal workspace preferences changed; configuration, music, library, bindings,
migrations and tool blockers are preserved, with no residual probes.
Removal of both `/tmp/v4vmm-startup-bxg5v6ic` and
`/tmp/v4vmm-startup-1oj2crwd` is confirmed. Recovery cleanup succeeded on retry
with `python3`. Closing the app and removing its isolated workspace satisfy
scratch-query cleanup, without a separate pre-quit UI-clear claim. The operator
also confirmed removal of `/tmp/v4vmm-selection-crlf-p1TRT7.txt`. Temporary
source checkout/copy and bundle removal was verified; cleanup is complete.

The added tests cover exact Copy, replacing the selected emoji, atomic Undo/Redo,
joined and skin-tone emoji, flags, keycaps, variation selectors, an Indic cluster,
every scalar within those samples, LF/CRLF rows and a 4,096-character rope prefix.
They do not prove native font shaping, Wayland, IME composition or all pointer
drag behavior. Before submission:

- Measure allocation/time on a very long single logical line. The previous
  selector scanned a bounded character window; the candidate may copy an
  entire line when it spans rope chunks. The long-line test proves correctness,
  not acceptable cost.
- Review double-click-and-drag, soft-wrap boundaries, scrolled rows, masked
  inputs, code-link click handling and non-left gestures against current main.
- If the proposed PR revision changes, repeat the accepted X11 cases, checking
  exact selected text, highlights, separate Copy/PRIMARY, replacement/Undo/Redo
  and the existing Escape path against that revision.

ADR 0072 authorizes only the documented gpui-base correction. The dependency
guard, lockfile and task record now identify the adopted commit. All other
GPUI dependencies retain their published pins. No upstream submission is
authorized. V1l's two failures are resolved by the pinned correction and its
accepted X11 checks. Final preservation and cleanup passed; the application
packet is complete. The review work above applies to a future upstream PR,
not to the accepted application packet.

## Operator Visual Check

The accepted run is complete and its fixtures were removed. For a future
correction revision:

1. Use the [correction retry](../runbooks/text-selection-check.md#adr-0072-correction-retry)
   to build the app and create a fresh fixture from a desktop terminal.
2. Open toolbar search with Ctrl+F and repeat cases A and B above. Use X11 and
   xfce4-terminal with xclip installed to match the recorded environment. No
   broadcast service or audio hardware is needed. Missing selection, a partial
   payload or included punctuation is a failure, even if the highlight looks right.
3. Do not submit the search or save configuration edits. Remove the scratch
   query before final preservation inspection. Retain a failed fixture for
   diagnosis; after a passing inspection, remove that run's fixture and scratch
   artifacts using the selection runbook.
