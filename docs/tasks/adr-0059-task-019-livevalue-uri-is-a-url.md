# ADR 0059 Task 019: The liveValue URI Is A URL

Status: Implemented - 2026-10-24. V1 to V4 are open, in the final walk of the run of 2026-10-24.

## Goal

The feed tag of a broadcast event names the relay that holds the event. The app writes
`<podcast:liveValue uri="https://<relay host>/event?event_id=<EVENT_ID>" protocol="socket.io"/>`
in each place where it shows or copies the tag. A listener app can then connect to the relay,
and Stophammer can match the `uri` to a confirming relay.

## Authority

- [ADR 0059](../adr/0059-broadcast-control-surface.md), amendment of 2026-09-06: the `Event`
  section shows the ready-to-paste `podcast:liveValue` tag with a copy action. This packet
  amends the form of that tag.
- [The live value URI plan](../plans/archive/livevalue-uri-is-a-url.md) of 2026-09-26. It records the
  defect and the correction. It states no rule.
- [ADR 0063](../adr/0063-show-dashboard-layout.md): Event Logs shows the tag in its diagnostics.
- [Broadcast chain delivery order](../plans/broadcast-chain-delivery-order.md), row 3: the relay
  adoption comes after this packet (overhaul plan, remaining order, step 3).
- [AGENTS.md](../../AGENTS.md): "Treat RSS and MusicIndex responses as untrusted input", "Every fix
  gets a guard" and "Typed action state".

## Recorded Facts - 2026-10-24

### The tag today

- `src/view_models/show.rs:2574-2579` `feed_tag_for_event(event_id)` writes
  `<podcast:liveValue uri="{}" protocol="socket.io"/>` with the escaped event identifier only.
  The plan fact is correct.
- `src/view_models/show.rs:2581-2593` `escape_xml_attribute` escapes `&`, `"`, `'`, `<` and `>`.
- The function has three callers. Each one passes `event.event_id` only:
  1. `src/view_models/show.rs:2310-2313`, `EventSectionDisplay::from_input`. The value goes to
     `EventSectionDisplay.feed_tag` (`:848`). The Event menu item "Copy feed tag" copies it
     (`src/ui/composites/show_detail_panel.rs:538-557` and `:604`).
  2. `src/view_models/show.rs:5985-5992`, the Event Logs pane. The value goes to
     `ShowLogPaneDisplay.feed_tag` (`:645`). The Logs header button "Copy feed tag" copies it
     (`src/ui/composites/show_log_pane.rs:176-188`).
  3. `src/view_models/show/event_report.rs:128-129`. The Event diagnostics text shows the tag
     on the line after "Feed tag for event ...".
- The menu action `copy_feed_tag` is available when an event is selected
  (`src/view_models/show.rs:2481` `has_event`, `:2523-2528`). The Logs button exists when
  `feed_tag` is `Some` (`:5989-5992`).
- `src/app/show.rs:2838` gives `EventControlIntent::CopyFeedTag` no command. The renderer
  writes the tag to the clipboard. No other source file writes `podcast:liveValue`.

### The tests that expect the old form

- `src/view_models/show.rs:4244-4247`, in `event_section_projects_attached_target_and_feed_tag`
  (`:4199`). This is the XML escape test. The event identifier `event&one` gives
  `uri="event&amp;one"`.
- `src/view_models/show/event_report.rs:365`, in
  `event_report_times_subjects_and_idle_rows_are_honest` (`:341`). It expects
  `uri="event-one"`.
- `src/view_models/show.rs:5001` and `:5565` examine only that the tag contains the identifier
  and that the diagnostics contain the tag. They pass with the new form.

### The relay host of an event

- Each stored event has a relay endpoint. `broadcast_events.endpoint` is `NOT NULL`
  (ADR 0059 task 002), and `src/db.rs:2027-2029` `explicit_broadcast_endpoint` rejects an
  empty value at insert. `BroadcastEventRow.endpoint` is `src/db.rs:312`.
- The app copies the row to `EventSelectionInput.endpoint` (`src/app/show.rs:1801-1812`,
  `src/view_models/show.rs:1108-1109`). The view model already holds the relay endpoint of the
  selected event. The Event diagnostics show it: "App checks this event at relay ..."
  (`src/view_models/show/event_report.rs:112`).
- At create, the endpoint is the configured `musicindex_endpoint`. The app reads it at
  `src/app/show.rs:1182-1184`. The CLI reads it at `src/cli.rs:215-221`.
  `BroadcastRegistry::with_clock` normalizes it with `config::normalize_musicindex_endpoint`
  (`src/broadcast/registry.rs:230-243`). `create_event` stores it (`:137`).
- `config::normalize_musicindex_endpoint` (`src/config.rs:1066-1087`) accepts only `http` and
  `https` with a host. It keeps a path, and it does not remove user information.
- The configuration has no separate relay key. The packet needs none, because the event row
  holds the relay of that event.
- The create response has an optional `socket_io_url` (`src/api.rs:853`). The relay sends the
  relative path `/event?event_id=<event_id>` (`musicindex-live-relay` `src/lib.rs:751`, commit
  `3764bac`). The app does not store it. The relay document `docs/interoperability.md:53` says
  that `v4vmm` stores the create response fields. That statement is not correct for this app.
- The relay makes an event identifier with `random_urlsafe` (`musicindex-live-relay`
  `src/lib.rs:2079`). The relay registers the Socket.IO namespace `/event` at the root
  (`src/lib.rs:1970`, `:3494`).
- An event that the operator reserved from a terminal is not in the app registry (delivery
  order, "A `v4vmm` packet for reserving a durable live item"). The app cannot show a tag for
  it. The runbook gives the form for that case.

### Documents that show the old form

- `docs/adr/0059-broadcast-control-surface.md:368-372`.
- `docs/runbooks/broadcast-operations.md:41-53` and step 3 at `:153`.
- `docs/runbooks/broadcast-event-recovery-check.md:155-156` expects
  `<podcast:liveValue uri="fixture-event-2" protocol="socket.io"/>`. The plan does not name
  this runbook. The fixture relay endpoint is `http://127.0.0.1:17863` (`:108`).
- The archived packets 005 and 017 also show the old form. They are history and do not change.

## Design

### The URI builder

A new module `src/broadcast/live_value.rs` owns the URI. It has one pure function:

```rust
pub(crate) fn live_value_uri(endpoint: &str, event_id: &str) -> Result<String, LiveValueUriError>
```

The function parses `endpoint` with `reqwest::Url`. It does these steps:

1. It refuses a scheme other than `http` or `https`, an endpoint with no host, and an empty
   event identifier.
2. It removes the user information, the path, the query and the fragment.
3. It sets the path `/event`.
4. It adds the one query pair `event_id=<EVENT_ID>` with `query_pairs_mut().append_pair`. This
   call encodes the value.

`LiveValueUriError` is a typed enum with the values `NoHost`, `UnsupportedScheme`,
`InvalidEndpoint` and `EmptyEventId`. The function does no I/O.

### The tag

`feed_tag_for_event` takes the `EventSelectionInput` and returns `Option<String>`. It calls
`live_value_uri(&event.endpoint, &event.event_id)` and escapes the result with
`escape_xml_attribute`. On an error, it returns `None`. The three callers pass the event and
keep their `Option`. `EventSelectionInput` gets no new field.

### The action state

The menu action `copy_feed_tag` is available only when the tag is `Some`. When an event is
selected and its tag is `None`, the action is unavailable. Its accessibility label says
"Copy listener feed tag. The stored relay endpoint of this event has no host." The Logs button
keeps its rule: it exists only with a tag. The diagnostics line says "App cannot make the feed
tag for event EVENT_ID, because its relay endpoint ENDPOINT has no usable host." See Q1.

## Decisions

- **D1.** The relay host comes from `broadcast_events.endpoint` of the event. Reason: the event
  exists only at the relay that created it. The current `musicindex_endpoint` can name a
  different relay after a configuration change.
- **D2.** The URI keeps the scheme, the host and the port of the endpoint. It removes the path.
  Reason: the relay registers the namespace `/event` at the root, and a Socket.IO client reads
  the URI path as the namespace. A path prefix gives a namespace that the relay does not have.
- **D3.** The URI removes the user information of the endpoint. Reason: a feed is public. A
  user name or a password in an endpoint must not go into it.
- **D4.** The event identifier and the endpoint are untrusted input. The builder encodes the
  `event_id` query value, and the tag escapes the full URI as an XML attribute. Reason: the
  identifier comes from a relay answer and the endpoint comes from a database row. Neither one
  can close the attribute or add a second query pair.
- **D5.** No new configuration key, no schema change and no migration. Reason: each event row
  already holds its relay endpoint.
- **D6.** The app does not store `socket_io_url` from the create response. Reason: the relay
  sends a relative path, and the app can make the same path from the endpoint. A stored copy
  needs a schema change for no new fact.
- **D7.** The builder lives in `src/broadcast/`, not in the view model. Reason: the relay
  address is a broadcast service fact. The view model already imports `crate::broadcast`
  (`src/view_models/show.rs:18`), and no view model file imports `reqwest` directly.

## Questions

- **Q1.** What does the app show when the stored endpoint of the selected event gives no usable
  host? Each new row has a valid endpoint, so only a changed or damaged row can give this
  result.

  - A: no tag. The menu action is unavailable, its accessibility label says why, and the
    diagnostics give the reason and the stored endpoint.
  - B: the app uses the current `musicindex_endpoint` in place of the stored endpoint. This can
    name a relay that does not hold the event.
  - C: the app writes the old form with the identifier only. This is the defect.

  Recommendation: A. A tag that names the wrong relay or no relay reaches no listener, and the
  operator gets no sign of the error. With A, the operator sees the cause. The operator
  replaces the event, or the operator corrects the row.

## Required Changes

All of these changes go in one commit.

1. `src/broadcast/live_value.rs` (new): `live_value_uri`, `LiveValueUriError` and their unit
   tests. The module comment names ADR 0059 and this packet.
2. `src/broadcast/mod.rs`: declare `pub mod live_value;`.
3. `src/view_models/show.rs`: change `feed_tag_for_event` as in "The tag". Change its callers
   at `:2310-2313` and `:5985-5992`. Make `copy_feed_tag` (`:2523-2528`) depend on the tag, with
   the accessibility label of "The action state". Change the expected value at `:4244-4247`.
4. `src/view_models/show/event_report.rs`: change the caller at `:128-129` and add the
   diagnostics line of "The action state". Change the expected value at `:365`.
5. `tests/architecture_tests.rs`: add the guard R19-07.
6. `docs/adr/0059-broadcast-control-surface.md`: add a dated amendment note to the Status
   section, after the note of 2026-10-18:

   > Amended 2026-10-24: the `podcast:liveValue` tag names the relay of the event. Its `uri` is
   > `https://<relay host>/event?event_id=<EVENT_ID>`, made from the stored endpoint of the event.
   > A tag with the identifier only names no relay, so no listener app can connect. Stophammer
   > found the defect in the work on its ADR 0064. [Task 019](../tasks/adr-0059-task-019-livevalue-uri-is-a-url.md)
   > builds it.

   Replace the XML example at `:368-372` with the new form. Add one sentence that names the
   relay host source (D1) and the guard R19-07.
7. `docs/adr/README.md:68`: add "The amendment of 2026-10-24 makes the liveValue URI a URL,
   built by task 019" to the scope of ADR 0059.
8. `docs/runbooks/broadcast-operations.md`: replace the example at `:47-49` with
   `<podcast:liveValue uri="https://api.musicindex.org/event?event_id=EVENT_ID" protocol="socket.io"/>`.
   Say that the host is the relay endpoint of the event, from `v4vmm broadcast events list
   --json`, without its path. Say that the operator builds this form by hand for an event that
   the operator reserved from a terminal. Change step 3 at `:153` to "Update the `uri` of the
   RSS `podcast:liveValue` tag with the new `EVENT_ID`."
9. `docs/runbooks/broadcast-event-recovery-check.md:155-156`: expect
   `<podcast:liveValue uri="http://127.0.0.1:17863/event?event_id=fixture-event-2" protocol="socket.io"/>`.
10. `docs/pending-human-checks.md`, section 7: add this packet with V1 to V4.
11. `docs/reviews/orchestrated-run-2026-10-24.md`, order step 6: link this packet.
12. After the commit of the packet, the run record or the operator moves
    `docs/plans/livevalue-uri-is-a-url.md` to `docs/plans/archive/`. In the same change, remove
    the line about the plan from "Known open defects" in `AGENTS.md`. Also change step 3 of the
    remaining order in `docs/plans/design-and-cleanup-overhaul-plan.md:179` to point at this
    packet.

## Mechanical Acceptance Criteria

Each criterion here is mechanical. A unit test beside its owning code has the prefix
`adr_0059_task_019_`. The guard is in `tests/architecture_tests.rs` and has its case number in
its name.

| Case | Required proof |
|---|---|
| R19-01 | Mechanical, unit test in `src/broadcast/live_value.rs`. `live_value_uri("https://api.musicindex.org", "abc_DEF-123")` gives `https://api.musicindex.org/event?event_id=abc_DEF-123` |
| R19-02 | Mechanical, unit test, same file. The endpoint `http://127.0.0.1:17863` gives `http://127.0.0.1:17863/event?event_id=...`. The scheme and the port stay |
| R19-03 | Mechanical, unit test, same file. The endpoint `https://user:secret@relay.example:8443/api/v1?x=1#f` gives `https://relay.example:8443/event?event_id=...`. The result contains no `user`, no `secret`, no `/api` and no `x=1` |
| R19-04 | Mechanical, unit test, same file. The identifier `a&b=c d#"<>` gives one query pair. The value is encoded, and the URI contains no `&`, no space, no `#` after the query start, no `"`, no `<` and no `>` |
| R19-05 | Mechanical, unit test, same file. `ftp://relay.example`, `https://`, `not a url` and an empty identifier each give their `LiveValueUriError` value |
| R19-06 | Mechanical, unit tests in `src/view_models/show.rs` and `src/view_models/show/event_report.rs`. With the endpoint `https://relay.example` and the identifier `event&one`, `EventSectionDisplay.feed_tag`, `ShowLogPaneDisplay.feed_tag` and the diagnostics line each equal `<podcast:liveValue uri="https://relay.example/event?event_id=event%26one" protocol="socket.io"/>`. The changed tests at `:4244-4247` and `event_report.rs:365` prove the first and the third |
| R19-07 | Mechanical, guard. Situational: it cites the ADR 0059 amendment of 2026-10-24 and this packet, and is deleted when that amendment is superseded. In `src/`, only `src/view_models/show.rs` contains `podcast:liveValue`. The body of `feed_tag_for_event` calls `live_value_uri`. Its failure message names ADR 0059, this packet and the fix |
| R19-08 | Mechanical, unit test in `src/view_models/show.rs`. A selected event with the endpoint `ftp://relay.example` gives `feed_tag` `None` in the Event section and in Logs. `copy_feed_tag` is unavailable, its accessibility label names the endpoint fault, and the diagnostics contain the reason line |
| R19-09 | Mechanical. Each existing test of `src/view_models/show.rs`, `src/view_models/show/event_report.rs` and `src/ui/composites/show_detail_panel.rs` passes. This includes `:5001` and `:5565` with no change |

The failure message of R19-07 has this form:

```text
ADR 0059 amendment of 2026-10-24, task 019: the podcast:liveValue tag must name the relay
of the event. Build its uri with broadcast::live_value::live_value_uri in
feed_tag_for_event. Do not write the tag in another file.
```

## Visual Acceptance Criteria

Each criterion here is visual. Only a person can examine it. No test proves it.

- V1: Show, Live Metadata, Event, More, "Copy feed tag" copies the full URL form. The pasted
  text names the relay host of the event.
- V2: Event Logs shows the same full tag in its text under "Feed tag for event ...". The text
  stays legible and selectable on one line or with a horizontal scroll, as before.
- V3: the "Copy feed tag" button in the Event Logs header copies the same text as V1.
- V4: a podcast feed that the operator changed holds the new tag. The operator replaces each
  old tag outside the app. This is step 9 of the operator check.

## Exclusions

- No schema change, no migration and no new configuration key.
- No storage of `socket_io_url` (D6).
- No feed writer. The app does not write the feed (ADR 0059).
- No reservation of a durable event. That is the `v4vmm` packet of delivery order row 3.
- No check that the relay answers at the URI. The delivery order records that check as future
  work, with a `confirming_relay` question to the relay.
- No change to the relay repository. Its statement at `docs/interoperability.md:53` is a fact for
  the relay owner.
- No change to `src/view_models/library.rs`, `src/view_models/releases_everything.rs` or the
  Library list UI. Another packet changes them now.

## Files To Inspect

- [AGENTS.md](../../AGENTS.md) and the [source map](../architecture/source-map.md).
- `src/view_models/show.rs` (`feed_tag_for_event`, `escape_xml_attribute`,
  `EventSectionDisplay::from_input`, `EventActionsDisplay::from_state`, the Event Logs pane and
  their tests).
- `src/view_models/show/event_report.rs` (the diagnostics and the `example` fixture).
- `src/ui/composites/show_detail_panel.rs` and `src/ui/composites/show_log_pane.rs` (the two
  copy paths, which change only through the view model).
- `src/broadcast/mod.rs` and `src/broadcast/registry.rs`.
- `src/config.rs` (`normalize_musicindex_endpoint`) and `src/db.rs` (`BroadcastEventRow`).
- `src/app/show.rs` (`event_selection_input`).

## Checks

```bash
cargo test
cargo test --test architecture_tests
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
cargo check --all-targets
touch src/main.rs && cargo build --bin v4vmm
python3 docs/runbooks/test_broadcast_event_recovery_fixture.py
```

## Rollback

Revert the commit. The packet stores no data. A feed that already holds the new tag keeps it.
The new tag stays correct after a rollback, because only the app display changes.

## Operator Visual Check

Steps 1 to 7 use the local fixture of the event recovery check. They need a Linux desktop
session, Python 3.11 or newer, two terminals and the free port `17863`. They change no real
service and no normal configuration. Steps 8 and 9 use the normal app and the feed of a real
show. They need write access to that feed and a published event.

1. In terminal A, prepare the fixture and start its relay:

   ```bash
   cd /home/citizen/build/v4vmm
   cargo build --bin v4vmm
   nm -C target/debug/v4vmm | grep -c TestDispatcher
   task016_dir=$(mktemp -d /tmp/v4vmm-task016.XXXXXX)
   python3 docs/runbooks/broadcast-event-recovery-fixture.py setup "$task016_dir"
   python3 docs/runbooks/broadcast-event-recovery-fixture.py serve "$task016_dir"
   ```

   The `nm` command must give `0`. Keep terminal A open.

2. In terminal B, start the app with the fixture configuration:

   ```bash
   cd /home/citizen/build/v4vmm
   if task016_dir=$(python3 docs/runbooks/broadcast-event-recovery-fixture.py locate) &&
      python3 docs/runbooks/broadcast-event-recovery-fixture.py verify "$task016_dir"; then
     env XDG_CONFIG_HOME="$task016_dir/config" PATH="$task016_dir/bin:$PATH" \
       ./target/debug/v4vmm &
     task016_app_pid=$!
   fi
   ```

   **Wrong if:** Settings shows an endpoint other than `http://127.0.0.1:17863`. Close the
   window and do step 2 again.

3. Open Show, then Live Metadata. Click "Create" one time. Wait for `fixture-event-1` in the
   picker.

4. V1: click More, then "Copy feed tag". Paste the text into a text editor. Expect exactly:

   ```xml
   <podcast:liveValue uri="http://127.0.0.1:17863/event?event_id=fixture-event-1" protocol="socket.io"/>
   ```

   **Wrong if:** the `uri` has the identifier only, has no `/event?event_id=`, or names
   another host.

5. V2: open Event Logs. Find the line after "Feed tag for event fixture-event-1".
   **Wrong if:** that line is not the same text as step 4, or you cannot read or select it.

6. V3: in the Event Logs header, click "Copy feed tag". Paste the text into the editor.
   **Wrong if:** the text is not the same as step 4.

7. Close the fixture app. Stop the relay with Ctrl+C in terminal A. In terminal B, remove
   only the fixture directory:

   ```bash
   printf '%s\n' "$task016_dir"
   test -f "$task016_dir/task-016-fixture" && rm -r -- "$task016_dir"
   unset task016_dir task016_app_pid
   ```

8. Start the normal app with `./target/debug/v4vmm`. Open Show, then Live Metadata. Select the
   event of the show. Click More, then "Copy feed tag".
   **Wrong if:** the host of the `uri` is not the relay endpoint that
   `./target/debug/v4vmm broadcast events list --json` gives for that event.

9. V4: this step is outside the app. In each RSS feed of a show that holds a
   `podcast:liveValue` tag, replace the old tag with the tag of step 8. Publish the feed. Then
   examine the published feed:

   ```bash
   curl -s FEED_URL | grep -o '<podcast:liveValue[^>]*>'
   ```

   Replace `FEED_URL` with the address of the feed.
   **Wrong if:** the command gives a tag with `uri="EVENT_ID"` and no host, or gives no tag.
   Record the feed address and the time of the check.

Cleanup: step 7 removes the fixture and its event. Steps 8 and 9 change no app data. The feed
change of step 9 stays, because it is the correction. To undo it, put the old tag back into the
feed and publish it again.

## Orchestrator Answers - 2026-10-24

- Q1: option A. With no usable host, the app gives no tag. The copy action is unavailable, and its accessibility label gives the reason.
  The diagnostics give the reason and the stored endpoint. This follows the durable set: typed action state, and a surface that cannot act is not offered.
- D2: confirmed. A Socket.IO client reads the path of the URI as the namespace, and the relay registers the namespace `/event` at the root. Thus the URI drops the path of the endpoint. The operator check of a real listener client stays V-level evidence.

## Implementation Notes - 2026-10-24

- `src/broadcast/live_value.rs` owns `live_value_uri` and `LiveValueUriError`. The `reqwest`
  crate does not export the parse error type of `Url`, and this packet does not change
  `Cargo.toml`. Thus the helper `has_empty_host` examines the text of an endpoint that does not
  parse. With this helper, `https://` gives `NoHost`, and `not a url` gives `InvalidEndpoint`.
- `EventActionsDisplay::from_state` takes the new argument `feed_tag_ready`. The constant
  `COPY_FEED_TAG_NO_HOST_A11Y_LABEL` holds the accessibility label of "The action state".
- Without a tag, the diagnostics show the reason line in place of the heading
  "Feed tag for event ..." and the tag.
- The guard `adr_0059_broadcast_services_stay_gpui_free_and_use_api_client` refused
  `reqwest::` in each file of `src/broadcast/`. This packet did not record that fact. The
  orchestrator decided option A on 2026-10-24.
- With option A, the guard removes each exact `reqwest::Url` from a line and then looks for
  `reqwest::`. Each other item of `reqwest` stays a violation. The reason is that `reqwest::Url`
  is a parser that builds no client. The guard exists to stop relay clients outside `api::Client`.
- The self-test
  `adr_0059_task_019_broadcast_boundary_allows_only_reqwest_url` proves that
  `use reqwest::Url;` passes and that `use reqwest::blocking::Client;` fails.
- The unit tests with the prefix `adr_0059_task_019_` prove R19-01 to R19-06 and R19-08. The guard
  `adr_0059_task_019_r19_07_live_value_tag_names_the_relay` proves R19-07. The full suite proves
  R19-09.
- The runbook `broadcast-operations.md` also tells the operator where to find the relay address
  of an event that the operator reserved from a terminal. That event is not in the output of
  `v4vmm broadcast events list --json`.
- The tag of the fixture event in step 4 is
  `<podcast:liveValue uri="http://127.0.0.1:17863/event?event_id=fixture-event-1" protocol="socket.io"/>`.
