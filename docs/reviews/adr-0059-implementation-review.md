# ADR 0059 Implementation Review

## Status

Passed - 2026-09-09. ADR 0059 is ready to mark `Implemented`.

## Artifacts Reviewed

- [ADR 0059](../adr/0059-broadcast-control-surface.md)
- [ADR 0059 phase plan](../plans/adr-0059-broadcast-control-surface-phase-plan.md)
- [Broadcast chain delivery order](../plans/broadcast-chain-delivery-order.md)
- ADR 0059 task packets 001 through 015
- [Broadcast operations runbook](../runbooks/broadcast-operations.md)
- [Pending human checks](../pending-human-checks.md)
- `src/api.rs`
- `src/broadcast/`
- `src/runtime/broadcast_observation.rs`
- `src/runtime/broadcast_service_watch.rs`
- `src/view_models/show.rs`
- `src/ui/shells/show.rs`
- `src/ui/composites/show_card.rs`
- `src/ui/composites/show_detail_panel.rs`
- `tests/architecture_tests.rs`

## Review Findings

Two things came out of the review itself, and both are fixed.

**A derived `Debug` printed a broadcaster token.** `LiveItemCreateResponse` in
`src/api.rs` derived `Debug`, so any error log that formatted it would have
carried the token. ADR 0059 states that token text never reaches a log. The
type now has a manual `Debug` that redacts the field, and a guard holds it.

No guard had caught this, because every earlier guard looked at the database,
the user interface, and the command line. The type that carries the token in
memory was not on that list.

**Five new guards carried no class.** ADR 0061 asks each guard to say whether it
is durable or situational, so an archiver knows which guards die with the
decision that made them. All five now read `Situational ADR 0059`.

Secret handling may belong in the durable set, because it survives any design
change. ADR 0061 says adding to that set is a decision that needs its own
record, and none exists, so these stay situational. That is the open question,
not an oversight.

## Invariant Review

| Invariant | Result | Evidence |
|---|---|---|
| The app sends no metadata to the relay | Pass | `adr_0059_v4vmm_does_not_publish_live_metadata` |
| Token text stays out of the database, logs, and `Debug` output | Pass | `adr_0059_broadcast_event_schema_keeps_tokens_out_of_storage_and_ui`, `adr_0059_broadcast_token_text_has_single_storage_boundary`, and `adr_0059_live_item_create_response_debug_redacts_broadcaster_token` |
| `src/broadcast/**` is GPUI-free and does not build a `reqwest` client | Pass | `adr_0059_broadcast_services_stay_gpui_free_and_use_api_client` |
| `systemctl`, `journalctl`, and `ssh` stay in `src/broadcast/` | Pass | `adr_0059_publisher_service_control_boundary_is_broadcast_owned`, `adr_0059_ssh_transport_boundary_is_broadcast_owned`, and `adr_0059_show_screen_and_shell_do_not_call_service_processes` |
| Source kind names stay at adapter and config boundaries | Pass | `adr_0059_source_kind_literals_stay_at_adapter_boundaries` |
| The drop file is written only by the producer module | Pass | `adr_0059_mpv_drop_file_producer_boundary_is_broadcast_owned` |
| Broadcast sections and `QueueNowPlaying` stay separate | Pass | `adr_0059_show_broadcast_sections_stay_separate_from_queue_now_playing` and ADR 0060 guards that remove the old `Broadcast` frame |
| Encoder commands stay in the broadcast service layer | Pass | `adr_0059_stream_encoder_control_boundary_is_broadcast_owned` |
| The app never sends a song title to the encoder | Pass | `adr_0059_stream_encoder_control_boundary_is_broadcast_owned` checks that `-u` is absent |
| Publisher configuration changes run through publisher tools | Pass | `adr_0059_publisher_configuration_changes_use_publisher_tools` |
| Dead events are reported and not auto-replaced | Pass | Registry unit tests map `404` to `Dead`; the runbook requires an operator-created replacement |
| Blocking relay and service reads use runtime or service boundaries | Pass | `adr_0059_broadcast_observation_actor_is_runtime_owned` and service-watch tests |
| Show broadcast presentation has dark-mode parity | Pass | `adr_0059_show_shell_uses_semantic_colors_for_dark_mode_parity` |
| Show broadcast controls use VM-owned accessibility labels | Pass | `adr_0059_show_shell_consumes_vm_owned_accessibility_labels` |

## Visual Evidence

An agent did not run the GPUI app. The operator cleared the inherited visual
gates in a desktop session on 2026-09-08 and 2026-09-09. The owning task status
lines and delivery-order rows record the checks.

| Visual state | Result | Evidence |
|---|---|---|
| Live event state | Pass | ADR 0059 tasks 002, 003, and 004 were operator-cleared on 2026-09-08 |
| Dead event state | Pass | ADR 0059 tasks 002, 003, and 004 were operator-cleared on 2026-09-08 |
| Publisher not installed | Pass | ADR 0059 task 009 state matrix was operator-cleared on 2026-09-08 |
| Failed unit with reason | Pass | ADR 0059 task 009 showed `Reason: start-limit-hit` |
| Open log panel | Pass | ADR 0059 task 009 log panel opened, read, and closed |
| Remote host not reachable | Pass | ADR 0059 task 010 showed `Broken` as `Not reachable` |
| Non-zero readiness count | Pass | ADR 0059 task 012 count matched `broadcast readiness --json` |
| Event target attach and detach | Pass | ADR 0059 task 014 updated the `default` target |
| Stream encoder controls | Pass | ADR 0059 task 015 confirmed not-configured and `butt` connect/disconnect states |
| Show cards and panel | Pass | ADR 0063 tasks 002 and 003 were retested after card and panel fixes |

No human visual gate remains open in `docs/pending-human-checks.md`.

## Missing Tests And Residual Risk

- Screenshot files were not committed for this closure pass. The accepted visual
  evidence is the operator-cleared task status lines.
- ADR 0063 task 004 still owns the future log bottom-pane placement work.
- HIG backlog A7 through A9 still tracks result-message room, stale service
  state flashes, and temporary stream-action disappearance.
- `splitkit` still needs reserved live items. Until then, relay restart or idle
  expiry kills events and the runbook recovery path is required.
- A future `v4vmm` packet still needs the seventh publisher state for installed
  but not configured.

## Architectural Drift

ADR 0060 replaced the planned `Broadcast` frame with Show sections. That is
intentional drift, not a violation. ADR 0059 remains the technical contract for
the broadcast chain, while ADR 0060 owns the surface structure.

## Recommendation

Merge the closure. Mark ADR 0059 and its phase plan `Implemented` with this
review as the named artifact.
