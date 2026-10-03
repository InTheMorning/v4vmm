# ADR 0075 Task 039: Feed Check And Update Observation Retention

Status: Implementation, technical review, and mechanical checks complete - 2026-09-21.
Root accepted the code and tests after seven test corrections. The normal binary build is Green.
Packet 038 is complete. The Root Compatibility Review section below records the five applied instructions.
Visual gates remain open and paused. Do not request a visual batch.

## Goal

Retain provider observations from feed checks and explicit feed updates through the existing shared writer.
Keep committed receipts through batch reduction, ordinary failures, storage failures, cancellation, and the composed payment-route repair operation.
Preserve existing requests, field values, tag edits, update markers, and command presentation.

The converted roots are `CheckFeedStaleness`, `CheckSubscribedFeeds`, `ApplyFeedUpdates`, and `CheckFeedsAndRepairRoutes`.
Payment-route repair requests retain no observation. The operator deleted packet 044 on 2026-09-21.
This packet preserves earlier feed receipts through that existing repair command. It does not convert its requests.

## Authority And Dependencies

- [ADR 0075](../adr/0075-metadata-ownership-and-completeness.md), separate provider evidence and preservation before projection.
- [ADR 0065](../adr/archive/0065-payment-route-tag-repair.md), the authorized combined check, update, and route-repair workflow.
- [Phase plan](../plans/adr-0075-metadata-contract-phase-plan.md#packet-register), caller assignments and dependencies.
- [Packet 013](adr-0075-task-013-verified-snapshot-replacement.md), completed RSS completeness registry, replacement, and typed reads.
- [Packet 014](adr-0075-task-014-provider-observation-retention.md), the shared recorder, writer, receipts, and storage capsules.
- [Packet 038](adr-0075-task-038-library-reader-observation-retention.md), application error ownership and separate Library evidence retention.
- [Storage contract](../schema/adr-0075-provider-snapshot-storage.md), provider/request identities and transaction boundaries.
- [Request baseline](../notes/adr-0075-request-and-write-baseline.md), historical measurements and their limits.

Preparation inspected the working tree after packet 013 completed technical and mechanical review.
Packet 038 completed implementation, technical review, and mechanical checks after preparation.
Its final types and callbacks require compatibility review before this packet becomes Ready.
The operator requires individual field-policy acceptance. This packet proposes no new field policy.

## Files To Inspect

Read the authority documents and these owners before editing:

- [Agent rules](../../AGENTS.md) and the [source map](../../.github/copilot-instructions.md).
- [Feed commands](../../src/application/commands/feed.rs), all four converted roots, result types, aggregation, events, and cancellation checks.
- [Feed service](../../src/feed_service.rs), staleness lookup, explicit updates, observed detail fetch, and RSS merge.
- [Library callbacks](../../src/library/app_impl.rs), `check_feed_on_view`, `check_all_feeds`, and `apply_all_feed_updates`.
- [Library state](../../src/view_models/library.rs), evidence retention and feed-operation completion methods.
- [Command errors](../../src/application/errors/command.rs), packet 038's ordinary error type and existing storage failure variant.
- [Library queries](../../src/application/queries/library.rs), packet 038's receipt assembly and behavioral fixtures.
- [API client](../../src/api.rs), exact feed and track profiles and recorder-aware transport.
- [Observation types](../../src/provider_observation.rs) and [writer](../../src/db/provider_observations.rs).
- [Subscription service](../../src/subscribe_service.rs), observed RSS enrichment and exact resource inputs.
- [Legacy ingestion](../../src/identity_ingest.rs), existing feed and track persistence.
- [Metadata service](../../src/metadata_service.rs) and [tag boundary](../../src/audio_tags.rs), existing edit generation and writes.
- [Payment-route commands](../../src/application/commands/payment_routes.rs), composed repair results, cancellation, locks, and error variants.
- [Command presenter](../../src/presentation/async_command_presenter.rs), session rejection and callback delivery.
- [Architecture guards](../../tests/architecture_tests.rs), caller ownership, source boundaries, and feed workflow guards.

## Files To Change

| File | Permitted change |
| --- | --- |
| `src/feed_service.rs` | Pass the command recorder through existing feed checks, detail fetches, and RSS merges. Preserve legacy persistence and tag behavior. |
| `src/application/commands/feed.rs` | Own the recorder, retain receipts in results and errors, and preserve evidence through composition and cancellation. |
| `src/application/errors/command.rs` | Attach observations to typed ordinary command failures without changing their display or cause. |
| `src/library/app_impl.rs` | Consume evidence before callback reductions. Extend exhaustive error detail handling for the new typed wrapper. |
| `src/view_models/library.rs` | Reuse packet 038's evidence stores and narrow storage status. Preserve feed-operation completion behavior. |
| `tests/architecture_tests.rs` | Add situational ADR 0075 guards for the four roots and their live consumers. |
| This packet | Record implementation, test evidence, request counts, and the prospective visual procedure. |

Unit tests stay beside their owners. Do not add another integration file or shared test module.
Do not edit packet 038's owners while its implementation is active.
Report any additional required owner before editing it.

## Existing Requests And Writes

These are inspected call bounds. Packet 016 did not measure these commands.

| Operation | Existing requests |
| --- | --- |
| Single feed check | Zero when the database returns no eligible feed row. Otherwise one feed request without `include`. |
| Subscribed-feed batch | One feed request for each eligible processed row, until an existing cancellation boundary. |
| Apply one feed | One initial feed request, then detail requests for each selected local track. |
| Normal track update | One scoped track request, one feed request, and at most one RSS request. |
| Scoped fallback | Adds one unscoped track request. When all Index requests fail, no RSS request follows. |
| Combined check | Feed-check requests, stale-update requests, then the existing separate payment-route repair requests. |

Update requests use exactly this include value:

```text
source_links,source_ids,source_release_claims,source_contributors,payment_routes
```

For `D` processed tracks, one feed application normally makes `1 + 2D` Index requests.
Its bound is `1 + 3D` Index requests and at most `D` RSS requests.
Configuration or database errors can stop the command earlier. Missing physical files can still incur requests before tag writes fail.
Keep missing local-path checks and scoped/unscoped fallback in their existing positions.

The database excludes null and empty feed GUIDs. It does not trim whitespace in those eligibility predicates.
The subscribed batch selects subscribed feeds. A direct feed-view check does not require subscription.
Track updates use Library membership joined with local-file rows. Do not change these predicates or their order.

Retain the existing Index path, query, requested subject, and separate profile.
Retain the existing RSS operation, resource, GUID, enclosure, and profile, including null versus empty inputs.
Do not add a caller name to request identity. Equal descriptors retain shared generation ordering.

Viewing a feed checks staleness. It does not apply metadata updates or write audio tags.
The explicit update stores supplied feed descriptions, legacy feed/track facts, generated tag edits, and the feed update marker.
These writes do not form one transaction. Earlier database and file changes can remain after a later failure.

The existing update marker advances after ordinary transport skips and tag-write errors.
Preserve that behavior. A storage failure stops dependent work before an unretained response can enter legacy persistence or tag generation.
Do not advance the current feed's marker after that new typed storage failure.
Earlier completed feeds and their existing markers remain unchanged.

## Recorder And Result Ownership

Use one recorder for each top-level execution of a converted root.
Factor the check and update loops into private command helpers that accept that recorder.
The combined command calls those helpers with its own recorder. Helpers do not drain receipts or execute duplicate requests.
Independent public commands create their own recorder and use the same helpers.

Remove compatibility wrappers when no production caller reaches them. Preserve their behavioral tests through the live shared helper.

Drain the recorder once, after the complete command result is known.
Place fallible configuration, database, request, merge, tag, and composition work inside that assembly boundary.
No `?`, cancellation return, or result projection after a request may bypass receipt assembly.

Attach receipts outside business values in these existing results:

- `CheckFeedStalenessResult`.
- `ApplyFeedUpdatesResult`.
- `CheckFeedsAndRepairRoutesResult`.

Review result C3 deletes `CheckSubscribedFeedsResult` with its dead command.

Do not add receipts to `StaleFeed`, API DTOs, tag edits, or legacy source facts.
Keep stale entries, counts, messages, route results, and emitted events equal for the same ordinary execution.
On failure, retain current event behavior. Retained observations do not invent successful completion events or roll back earlier writes.

## Typed Failures And Cancellation

The application error module owns a small `ObservedCommandFailure` wrapper.
It retains the original `CommandError` through `Arc<CommandError>` and committed receipts through `Arc<[ObservationReceipt]>`.
Add `CommandError::ObservedCommandFailure(Arc<ObservedCommandFailure>)`.
Its display delegates directly to the original cause. Its Debug output omits messages and private observation contents.

Keep clone and equality support. Expose only accessors used by error presentation, retention, or typed cancellation inspection.

Use one application-owned attachment function for ordinary errors and cancellation.
With no receipts, return the original error. With receipts, preserve the original typed cause and exact display.
Flatten existing observation wrappers. Preserve earlier receipts when attaching later receipts.

Reuse packet 038's query error payload when that payload already owns receipts.
Never recast a feed, cancellation, or route-repair error as `Query` merely to carry observations.

Classify storage failures before ordinary errors:

1. Preserve the existing write capsule or allocation error in `ObservationCommandFailure`.
2. Attach all committed receipts, including receipts from earlier feed operations.
3. Preserve any existing read error and earlier storage capsule.
4. Return the existing `CommandError::ObservationWriteFailure` variant.

Do not hide a typed storage failure inside the ordinary wrapper.
Do not stringify a capsule into `feed_errors` or discard it through `.ok()` or `continue`.
No provider module receives a `CommandError` or an arbitrary ordinary error string.

Ordinary batch-check errors remain skipped after their observations are retained.
Ordinary update errors retain their existing per-feed aggregation and messages.
Ordinary per-track transport failures and tag-write errors retain their existing continuation rules.
Storage failures stop the current top-level execution with a typed error and all earlier receipts.

Keep existing cancellation checks and their positions. Do not add a new cancellation policy inside a feed update.
Cancellation before any request returns the existing bare `Cancelled` variant.
Cancellation after committed observations returns a failed command with its original `Cancelled` cause and retained receipts.
The presenter continues to reject callbacks from a closed application session.
This packet makes no persistence claim for an uncommitted response lost during session teardown.

## Composed Route Repair Boundary

`CheckFeedsAndRepairRoutes` preserves ADR 0065's check, apply, then repair sequence.
Run the existing repair command inside the outer result assembly after the observed feed helpers.
Keep earlier feed receipts through repair success, ordinary failure, typed failure, and cancellation.
Retain existing feed-update and route-repair result fields and event ordering on success.

The current repair holds its database mutex across HTTP work. Attaching a recorder there would deadlock.
Do not edit that request path or release its locks.
Do not change repair eligibility or its tag-only edit filter in this packet.

Do not claim full combined-command observation coverage.
The operator deleted packet 044 on 2026-09-21. The repair's own requests keep their current behavior and retain no observation.

## Callback And View-Model Ownership

All three Library callbacks retain success receipts before reducing results into stale entries, counts, or messages.
All error callbacks retain receipts and capsules before formatting the error or changing selection-dependent state.
Use packet 038's generation-keyed stores. Preserve earlier evidence through navigation and later results.

Evidence retention alone must not change selection, removal confirmation, loading, busy state, or feed-operation state.
The existing feed completion methods still finish their own operation and clear their own in-flight markers.
Ordinary feed errors keep their existing feed status presentation.
Storage failures also use the established narrow Library storage status after their evidence is retained.
Do not call the broad `set_error_status` helper for retention.

Extend `command_error_detail` by delegating to the original cause for the ordinary wrapper.
Its returned detail must equal the detail for the original error.
No renderer invents error classification or metadata selection.

## Mechanical Acceptance Criteria

Use the prefix `adr_0075_feed_observation_` for behavioral tests beside the owning code.
Use disposable databases, scripted localhost services, and temporary audio files.

| Case | Required proof |
| --- | --- |
| F39-01 | Each real converted root allocates durable request generations before HTTP work. Success results retain every committed feed receipt. |
| F39-02 | Exact paths, includes, profiles, fallback order, request counts, and eligible-row filtering match the existing behavior. |
| F39-03 | Successful, malformed, failed, and unknown-member response evidence survives database reopen. RSS retains only the approved identity completeness contract. |
| F39-04 | Allocation failure prevents its request. Response-write failure prevents dependent legacy persistence and tag generation. |
| F39-05 | Earlier receipts survive later allocation, response-write, configuration, database, merge, and tag-write failures. Capsules remain typed. |
| F39-06 | Ordinary batch errors still skip or aggregate at their existing boundaries. Metadata values, generated edits, counters, messages, and events remain equal. |
| F39-07 | The feed-view root never writes tags. Explicit application preserves legacy values, source grouping, file changes, and update-marker behavior. |
| F39-08 | Cancellation before work creates no requests. Cancellation between feeds retains earlier receipts and the original typed cancellation cause. |
| F39-09 | The combined command retains feed receipts through route-repair success, failure, and cancellation without converting route requests. |
| F39-10 | Ordinary wrapper display and error detail equal the original command error. Storage errors retain their existing safe display and typed variant. |
| F39-11 | The three production callback boundaries retain evidence before reduction or selection checks. Navigation cannot discard earlier receipts or capsules. |
| F39-12 | Evidence retention preserves unrelated selection, loading, busy state, removal confirmation, and earlier failures through later results. |
| F39-13 | Ordinary feed completion behavior and suppression remain unchanged. Storage errors finish only the current operation and use narrow status ownership. |
| F39-14 | Repeated responses reuse shared evidence. Report legacy mutations, observation/snapshot mutations, and transactions separately. |
| F39-15 | Shared generation ordering holds across equivalent Library and update detail requests. Different profiles retain separate refresh state. |
| F39-16 | Existing snapshot, observation, Library retention, RSS, and feed workflow guards remain effective. No excluded caller becomes observed. |

For F39-05, include failures after one or more observations commit.
Inject a late legacy write failure and a missing-file tag failure without asserting rollback of earlier successful writes.
For F39-07, test ordinary skips and tag errors separately from storage failure before the current feed marker update.
For F39-09, exercise the real composition boundary. A helper that only concatenates synthetic receipts is insufficient.
For F39-11, exercise the production callback retention helper, not only an isolated database reader.

## Test Commands

Coordinate the final combined suite with root. Use direct Cargo commands.
Rebuild the normal desktop binary after all tests.

```bash
cargo test --locked --offline adr_0075_feed_observation_
cargo test --locked --offline adr_0075_library_observation_
cargo test --locked --offline adr_0075_observation_
cargo test --locked --offline adr_0075_snapshot_
cargo test --locked --offline adr_0075_rss_
cargo test --locked --offline --test architecture_tests
cargo check --locked --offline
cargo clippy --locked --offline -- -D warnings
cargo fmt -- --check
cargo test --locked --offline -- --test-threads=4
cargo build --locked --offline --bin v4vmm
python3 -B docs/runbooks/check-markdown-links.py docs/tasks/adr-0075-task-039-feed-check-and-update-observation-retention.md
python3 -B /home/citizen/.agents/skills/asd-ste100/scripts/ste_lint.py --check --no-heuristics docs/tasks/adr-0075-task-039-feed-check-and-update-observation-retention.md
git diff --check
```

## Exclusions And Escalation

Do not change schemas, migrations, provider contracts, decoders, request profiles, field policies, tag formats, or configuration formats.
Do not convert subscription, download, search, discovery, payment-route, or other caller families.
Do not add automatic retry, a visible retry action, a new global status surface, or renderer layout.
Do not launch the app, run headless, commit, change upstream code, or access production data.
Do not edit shared status documents. Root owns the ADR, plan, delivery order, and pending-check index.

Return to root for a broader error owner, changed continuation rule, new request, or required file outside the permitted set.
Also return if packet 038's final types conflict with the result contract above.
Do not invent a successful command result to preserve evidence from a failed operation.

## Rollback

A code revert leaves schema 12 and retained observations intact.
Do not delete evidence, reset generations, or undo unrelated working-tree changes.
Mechanical fixtures remove only their own files and databases.

## Expected Report

Report changed files, exact converted roots, request profiles, and result/error consumers.
Map F39-01 through F39-16 to behavioral tests and record commands, exits, and available output paths.
Report request counts, row mutations, and committed transactions separately.
Distinguish inherited writer coverage from new command and callback coverage.
State that payment-route requests retain no observation.
Record prospective visual steps, document links, language-check limits, and every remaining gate.

## Preparation Checks

The packet's local links passed inspection with the title rules, phase plan, and pending-check index.
The shared STE checker reports lexical findings and no structural findings after confirmed corrections.
The raw language result is not Green. Technical terms retain their source meaning.
No app, fixture server, Cargo command, upstream request, or production-data operation ran during preparation.

This task packet is the only new document. No folder or root document was created, and no file moved.
Root updated the existing phase plan and pending-check index separately.
Root completed that compatibility review on 2026-09-21. The next section records its results.

## Root Compatibility Review - 2026-09-21

Root inspected packet 038's completed types, the three live callbacks, and the existing guards.
These five results change the instructions above. Apply them.

### C1, The Query Wrapper Holds A Message

`ObservedQueryFailure` holds a `String` message and an `Arc<[ObservationReceipt]>`.
It holds no `CommandError`. Its display prefix is `query refresh failed: `.

Attach later receipts to that variant through a new `ObservedQueryFailure` with the same message.
Do not put that variant inside `ObservedCommandFailure`, because that moves its error family.
Flatten only an existing `ObservedCommandFailure`. Merge its receipts and keep its original cause.

### C2, The Combined Command Has An ADR 0065 Ordering Guard

`adr_0065_readiness_rows_keep_state_labels_separate_from_actions` requires three exact strings in order.
They are `CheckSubscribedFeeds::new`, `ApplyFeedUpdates::new`, and `RepairMissingPaymentRouteTags::new`.
The guard reads them inside `impl ApplicationCommand for CheckFeedsAndRepairRoutes`.

Private helpers remove the first string. Update that guard in the same change.
The new guard keeps the ADR 0065 owner, the check, apply, then repair order, and a message that names ADR 0065.
Do not name a helper to satisfy the old string. Do not delete this guard.

### C3, The Subscribed-Feed Command Loses Its Only Production Caller

`CheckSubscribedFeeds` reaches production only through `CheckFeedsAndRepairRoutes`.
A private helper makes that public command and its result type dead code.
Delete both, and move their behavioral tests to the live helper.
Report each removed item by name.
Keep `CheckFeedStaleness` and `ApplyFeedUpdates`. The Library callbacks call them directly.

### C4, Receipts Supply No Debug Output

`ObservationReceipt` implements `Clone`, `PartialEq`, and `Eq` only.
`CommandError` derives `Clone`, `Debug`, `Eq`, and `PartialEq`, so the new payload supports all four.
Write a manual `Debug` for `ObservedCommandFailure` that ends with `finish_non_exhaustive`.
`ObservedQueryFailure` shows the required form.

### C5, The Production Retention Helper Exists

`retain_library_query_failure` in `src/library/app_impl.rs` selects the retention path today.
Route the new wrapper through that helper. Keep the storage failure path on `retain_observation_failure`.
Extend `LibraryViewModel::retain_query_evidence` for the new variant.
The three feed error callbacks call `finish_feed_view_check_error`, `set_feed_check_error`,
and `finish_apply_feed_updates_error`. Retain evidence before each of those calls.

## Prompt for lower-context coding model

You are implementing one bounded task from a larger plan.

Implement only this task. Do not redesign the architecture.

Read:

- This packet, its authority documents, and all Files To Inspect.
- The shared STE and Rust development skills before applicable changes.

Goal:

- Retain feed-check and explicit-update observations through the shared writer and live Library consumers.
- Preserve evidence through failures, cancellation, and the existing route-repair composition.

Constraints:

- Wait for packet 038 completion, root compatibility review, and explicit dispatch.
- Preserve existing field, tag-write, marker, request, and ordinary continuation behavior.
- Keep storage errors typed. Keep the original cause and display of ordinary errors and cancellation.

Do not touch:

- Owners outside Files To Change or behavior in Exclusions And Escalation.
- Payment-route requests, upstream code, production data, or shared status documents.

Acceptance criteria:

- Prove F39-01 through F39-16 at the production command, composition, callback, and view-model boundaries.
- Retain evidence before result reduction and preserve all inherited guards.

Test commands:

- Use Test Commands. Coordinate the combined suite with root.
- Rebuild the normal desktop binary after tests. Check packet links and STE findings.

At the end, report:

1. files changed
2. tests run
3. behavior changed
4. deviations from task
5. unresolved concerns

## Implementation Evidence - 2026-09-21

The converted roots are `CheckFeedStaleness`, `ApplyFeedUpdates`, and `CheckFeedsAndRepairRoutes`.
Each root makes one recorder and drains it one time. The architecture guard counts both.
Correction C3 is applied. `CheckSubscribedFeeds` and `CheckSubscribedFeedsResult` are deleted.
The private helper `check_feed_batch_for_updates` keeps their behavior for the combined root.

The feed service keeps the existing requests. The staleness check calls
`client.fetch_feed(&stored.feed_guid, None)`. The update sequence keeps its four requests:
the feed profile, the track profile, the track feed profile, and the RSS body.
Their include string is unchanged.

The route repair keeps its own profile with the single `payment_routes` include.
It allocates no generation and retains no observation.
The repair holds the database lock during its HTTP work.
The behavioral fixture does not use the database while it answers those requests.

Legacy persistence, generated tag edits, update markers, counters, and messages are unchanged.
A storage failure stops its dependent legacy write and tag generation. Earlier receipts stay retained.
The three Library callbacks retain evidence before each existing error call.
Corrections C1, C2, C4, and C5 are applied. The ADR 0065 order guard now reads the private helpers.

### Changed Files

| File | Result |
| --- | --- |
| `src/feed_service.rs` | Passed the recorder through the checks, detail fetches, and RSS merges. Added storage-failure propagation. |
| `src/application/commands/feed.rs` | Converted the three roots, added receipt assembly, deleted the dead command, and added eight behavioral tests. |
| `src/application/errors/command.rs` | Added the ordinary observed command failure with private debug output and receipt attachment. |
| `src/library/app_impl.rs` | Routed the three feed callbacks through the existing retention helper. |
| `src/view_models/library.rs` | Extended evidence retention for the new variant. |
| `tests/architecture_tests.rs` | Added the packet guard and updated the ADR 0065 order guard. |
| This packet | Recorded implementation and verification evidence. |

No document moved, no folder was added, and no repository-root document changed during this assignment.

### Behavioral Evidence Map

All eight new unit test names start with `adr_0075_feed_observation_`. The table gives their suffixes.

| Criteria | Test evidence |
| --- | --- |
| F39-01, F39-02, F39-15 | `check_roots_allocate_generations_before_requests`. The fixture reads a committed pending slot before it answers. |
| F39-02, F39-07, F39-14 | `update_preserves_requests_files_and_markers`. Requests, tag edits, markers, and separate mutation counts stay equal. |
| F39-03 | `retained_bodies_survive_a_database_reopen`. Successful, malformed, failed, and unknown-member bodies survive. Only `source_ids` carries an RSS contract. |
| F39-04, F39-05, F39-10 | `storage_failures_stop_dependent_work`. Allocation failure prevents its request. Response-write failure prevents legacy persistence and tag writes. |
| F39-05, F39-06, F39-10 | `ordinary_failures_keep_skips_and_messages`. Skips, per-feed messages, error families, and safe display stay equal. |
| F39-08 | `cancellation_keeps_cause_and_receipts`. Early cancellation makes no request. Later cancellation keeps the typed cause. |
| F39-09 | `combined_root_keeps_receipts_through_repair`. The real composition runs through repair success, failure, and cancellation. |
| F39-10 | `receipt_attachment_keeps_error_families`. Attachment preserves each error family. |
| F39-11, F39-13, F39-16 | `adr_0075_feed_observation_roots_and_consumers_are_guarded`. The guard reads root ownership, service order, error contracts, and callback retention before reduction. |
| F39-12 | `adr_0075_library_observation_receipt_retention_preserves_confirmation_loading_and_busy_state`. Retention preserves unrelated Library state. |

### Root Verification Corrections - 2026-09-21

Root ran the mechanical suite and corrected seven defects in the new test code.
The production code needed no correction. Each item below records the measured behavior.

| Defect | Correction |
| --- | --- |
| The fixture asserted a committed generation for every request. The route repair holds the database lock during HTTP work, and its request made a deadlock. | The fixture identifies the repair by its single `include=payment_routes` value and answers it without the database. |
| The first guard text matched the converted profiles, because they also request payment routes. | The guard reads the exact repair query string. |
| The combined test expected one request. | The test records the three real requests and names their owners. |
| A query read `body_key` from `metadata_bodies`. That column does not exist. | The test compares retained bodies with the sum of observation occurrences. |
| A query read `provider_id` from `metadata_coverage`. That column does not exist. | The test joins through `metadata_observations` and reads `contract_id` for the contract claim. |
| The update test expected an unchanged embedded title. The explicit update writes source values into the file. | The test records the written source title. The next steps still prove that a feed view writes no tag. |
| Two counts were estimates. The update sequence makes four requests, and the repair reports no tagged track. | The test records four receipts and the empty event list. |

### Commands And Results

| Command | Result |
| --- | --- |
| `cargo test --locked --offline adr_0075_feed_observation_` | Green. Eight unit tests and one guard. |
| `cargo test --locked --offline adr_0075_library_observation_` | Green. Eleven unit tests. |
| `cargo test --locked --offline adr_0075_observation_` | Green. Twenty unit tests. |
| `cargo test --locked --offline adr_0075_snapshot_` | Green. Eighteen unit tests. |
| `cargo test --locked --offline adr_0075_rss_` | Green. Thirty-three unit tests. |
| `cargo test --locked --offline --test architecture_tests` | Green. 269 tests. |
| `cargo check --locked --offline` | Green. |
| `cargo clippy --locked --offline -- -D warnings` | Green. |
| `cargo fmt -- --check` | Green after `cargo fmt` corrected this packet's files. |
| `cargo test --locked --offline -- --test-threads=4` | Green. 1,650 unit tests and 269 architecture tests. Ten documentation examples stay ignored. |
| `cargo build --locked --offline --bin v4vmm` | Green. The normal binary is newer than the test link. |
| `python3 -B docs/runbooks/check-markdown-links.py` on this packet | Green. 27 local links. |
| `ste_lint.py --check --no-heuristics` on this packet | Lexical findings remain, as Preparation Checks records. No structural finding. |
| `git diff --check` | Green. |

No application launch, production-data change, or visual acceptance occurred.
The presentation gate below stays open and paused.

## Operator Visual Check

The presentation gate remains open and paused. Do not run or request these checks during the pause.
The implementation agent must supply exact terminal steps before offering this gate.
Extend [packet 014's isolated fixture](adr-0075-task-014-provider-observation-retention.md#isolated-desktop-procedure) using the completed behavioral tests' response scripts.
Use a Linux desktop, Python 3, temporary audio files, and the rebuilt normal binary.
No failed production service or special hardware is required.

1. Create the disposable fixture and start its isolated Index/RSS response server.
2. Open its Library album and verify the existing feed-check state.
3. Inject delayed observation storage failure in the feed-check response, then navigate to a different track.
4. Verify retained failure status without changed selection, metadata, or unrelated loading state.
5. Clear the failure and run the existing explicit feed-update action against the fixture files.
6. Inject a later storage failure after an earlier update observation commits.
7. Verify that the feed operation ends without claiming successful persistence for the failed response.
8. Clear the failure and run **Check all feeds** through its existing combined workflow.
9. Verify that ordinary results and feed-operation controls keep their existing placement and behavior.
10. Close the fixture app, stop its server, remove injected state, and remove only the disposable fixture.

Mechanical checks cannot close this gate or the inherited packet 012 and 014 gates.
