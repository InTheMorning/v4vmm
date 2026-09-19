# ADR 0069: Settings And Presets Phase Plan

## Status

Design accepted - 2026-09-11. [Task 001](../tasks/adr-0069-task-001-grouped-settings-foundation.md)
is complete with mechanical checks Green, operator V1–V3 and preservation
acceptance, and confirmed fixture cleanup. Its [operator procedure](../runbooks/settings-foundation-check.md)
remains a regression check. Later rows define phases. They are not implementation packets.

Reconciled 2026-09-18: recovery tasks 005–007 are complete with acceptance.
Phase 002 now needs its implementation packet. Later phases retain the
configuration-format and audio prerequisites below.

## Goal

Deliver grouped Settings, shared guarded editing, live metadata resource
selection and selective preset save/recall under
[ADR 0069](../adr/0069-grouped-settings-and-selective-presets.md).
Keep playback implementation deferred under proposed ADR 0068.

## Non-Goals

No whole-file configuration migration, workspace key merge, library relocation,
new audio engine, preset inheritance, arbitrary field masks or remote preset
sharing. Do not mix unrelated pending UI polish into the Settings foundation.

## Current State And Assumptions

- `src/config.rs` owns scoped configuration readers and ordinary persistence.
  `TopApp` owns Settings inputs, saving and existing runtime updates.
- ADR 0066 tasks 001–003 and 005–013 are complete with their required acceptance
  and cleanup. Task 004 retains its operator gate. Task 005 used the operator's
  explicit scheduling exception. Phase 002's named recovery prerequisites are complete.
- ADR 0066 tasks 005–007 own session transitions, guarded shared correction and
  optional retry. Reuse those owners instead of constructing a Settings-only
  transaction engine.
- The delivery order requires the complete ADR 0066 series before any config
  format change. This includes new persisted mode/resource keys and presets.
  Their design is accepted here; their persistence is not yet authorized for
  implementation ahead of that prerequisite.
- Existing service control and Event registry authority remain under
  ADRs 0059/0063. No service is created or configured by a navigation action.

## Target State And Affected Modules

| Owner | Responsibility |
|---|---|
| `src/config.rs`, later focused configuration modules | One scoped parse/validate/preserve/write authority; versioned component serialization |
| `src/view_models/settings.rs` | Existing group navigation and field/action presentation. Later phases add drafts, masks and changes. |
| `src/app/settings.rs` | Existing Settings composition and focus/input wiring |
| `src/ui/composites/settings.rs` | Existing shared forms and navigation geometry |
| Existing application commands and ADR 0066 maintenance owners | Validation, save, apply and retry with typed results and stale-result rejection |
| Existing Show view models, Event registry and broadcast adapters | Shared resource selection and operational command authority |
| ADR 0068 playback owners/adapters | Independent Show and audition routes; future audio Settings consumers |

Task 001 created the Settings owners above. Later packets must extend those
owners and the shared recovery commands. Create a module only when the active
packet supplies its caller.

## Sequence And Stopping Points

The [cross-repository delivery order](broadcast-chain-delivery-order.md) owns
priority. Each row requires its own bounded packet and verification before
implementation. Finish one packet per session.

| Phase | Usable result | Prerequisite | State |
|---|---|---|---|
| [001: Grouped Settings foundation](../tasks/adr-0069-task-001-grouped-settings-foundation.md) | Existing controls grouped under General, Library and Diagnostics; persistent in-session navigation/inputs and direct report routing | Accepted ADR 0069; existing app and scoped config owners | Complete - 2026-09-11; mechanical checks Green; V1–V3 and preservation accepted; fixture cleanup confirmed |
| 002: Shared guarded editor | One draft/save/cancel contract, field errors and saved/running distinctions through shared recovery commands | Task 001 and recovery tasks 005–007 are complete with acceptance. | Implementation not started. Prerequisites met. Author the packet against the delivered owners. |
| 003: Live metadata setup | General mode selection and Live Metadata producer/publisher editors, compatible defaults and explicit apply behavior | 002 accepted; full ADR 0066 configuration-format prerequisite released | Not started; author schema and bounded packets before edits |
| 004: Selective presets | Versioned named snapshots; save/recall masks for delivered components, composition in a draft, change review and conflict-safe persistence | 003 accepted; shared guarded persistence ready | Not started; split persistence/recall model and UI into separate packets if needed |
| 005: Independent audio settings | Audio tab and Show/Audition preset components, PulseAudio and JACK destination controls backed by independent owners | Guarded editor and relevant preset model; ADR 0068 accepted and owner/route isolation delivered and verified | Deferred with playback; no implementation packet yet |

The initial preset UI lists only implemented components. Show/Audition audio
checkboxes arrive with their adapters, not as disabled promises. Native PipeWire
is a later adapter packet, not part of the first audio delivery.

Task 001 completed independently of ADR 0066 task 004's playback gate.
Phase 002 can proceed after its packet defines the scope and checks.
Phase 003 still requires full ADR 0066 acceptance. Task 004 prevents that
release while its playback checks remain paused for ADR 0068 and the mpv IPC error.

Any narrower configuration-format prerequisite needs an explicit decision before
new configuration writes. The completed editor prerequisites do not grant that exception.

## Schema And API Implications

Task 001 adds no TOML key, table, database schema or preset document. Group
selection is session-local. Existing field saving and global Use Defaults keep
their existing meaning during this composition-only transition.

Later schemas retain the current active config document and store presets
separately. Component definitions are shared by config and preset validators.
The persistence packet must specify stable preset IDs, schema version, exact
file layout, known-component decoding, concurrent edit handling and compatibility
fixtures before implementation. Resolve external resource identities using
existing domain owners; do not invent a second Event registry or token store.

## Risks And Test Strategy

| Risk | Required proof |
|---|---|
| Group changes lose edits, focus or reports | Settings VM/dispatch tests plus operator navigation, narrow-width and report-route checks |
| UI reorganization becomes a second save implementation | ADR 0069 situational ownership guard and command-call review |
| Partial recall clears fields or silently repairs incompatible combinations | Component merge tests with absent/unchecked/overlapping values and cross-component validation failures |
| Save or recall changes an active service/player | Recording command/resource fakes proving no operational dispatch until explicit apply |
| External edits, unavailable resources or newer presets destroy valid setup | Shared writer conflict/failure tests; unsupported-version and missing-resource tests |
| Audition reaches program audio | ADR 0068 owner/route tests and operator capture/listening proof; outside the first packet |

The [review checklist](../reviews/adr-0069-settings-presets-review-checklist.md)
maps the full decision to proof. Each packet names its mechanical assertions
and separately supplies operator commands, prerequisites and cleanup before a
visual gate opens. Existing pending checks remain open unless their specific
requirements are walked and recorded. Settings changes may share evidence with
the surviving ADR 0030 Settings scroll check; they cannot close Music scrolling.

## Rollback Strategy

Revert the failing packet's code as a coherent change. Preserve configuration,
presets, private backups, database records and music. Never activate services or
change desktop audio routes as part of code rollback. Later schema packets must
state old-binary behavior before shipping their first write.

## Open Questions For Later Packets

- Exact versioned preset file schema and external resource ID representation
  belong to phase 003/004 packet design after the shared writer exists.
- Service apply transitions and conflict reporting must be specified against
  actual producer/publisher adapters before phase 003 implementation.
- Backend enumeration and JACK channel/port presentation require the concrete
  ADR 0068 adapter contract before phase 005.

These do not block task 001 and are not permission to improvise their later
architecture during implementation.
