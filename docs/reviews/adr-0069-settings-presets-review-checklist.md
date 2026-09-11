# ADR 0069: Settings And Presets Review Checklist

## Status And Scope

Planning artifacts recorded - 2026-09-11. Implementation not started.
No mechanical or visual implementation acceptance is claimed. Review each
packet against [ADR 0069](../adr/0069-grouped-settings-and-selective-presets.md),
the [phase plan](../plans/adr-0069-settings-presets-phase-plan.md) and its diff.

Task 001 is ready for implementation. Later phases require their own packets
and completed dependencies. This checklist supplies review coverage, not a
claim that future test names or controls exist.

## Decision Coverage

| Invariant | Mechanical proof required | Operator proof when delivered |
|---|---|---|
| 1. Inert group navigation | VM/dispatch retains edits and selected group; no saves or resource calls | Task 001 V1/V2: tabs, focus, scroll and unsaved text |
| 2. Explicit component changes | Mode/default and mask tests retain every unselected component; incompatible combinations explain conflicts | Later metadata/preset packet: review compatible and conflicting combinations |
| 3. Draft/saved/running distinction | Recall/save fakes perform no Start/Stop/Attach/audio calls; stale apply rejection and failed-apply state tests | Later editor/resource packet: draft, saved and running effect explanations |
| 4. One guarded writer | Shared Settings/recovery commands; identity/revision, backup, validation and conflict preservation tests | ADR 0066 editor checks plus Settings integration proof |
| 5. Snapshot recall | Save mask, absent/unchecked state, sequential overlap, immutable source semantics and Custom/provenance tests | Later preset packet: assemble components from two named presets |
| 6. Excluded sensitive/session data | Serialization allowlist and fixture inspection exclude secrets, cue/session state, library identity and health | Preset inspection after operator save/recall; no service activation |
| 7. Independent audio scope | Backend-specific destination decoding, missing-resource preservation, ADR 0068 owner and route tests | Deferred: separate program/monitor capture, missing-output behavior |
| 8. Live typed UI owners | Situational ADR 0069 ownership guard; reachable modules, typed actions and tokens | Each delivered surface checked at normal/narrow widths and both themes |

## Task 001 Review

- Verify only General, Library and Diagnostics are introduced. All existing
  settings, report and cached-file actions remain reachable.
- Trace field/action facts to the view model and geometry to shared UI owners.
  Reject renderer-side availability checks or parked future modules.
- Trace Save and Use Defaults back to the existing writer. Both retain their
  existing whole-form scope, visibly described across the editable groups.
  Do not claim the later guarded editor or Cancel/Apply behavior has shipped.
- Check Open report chooses Diagnostics in one action and ordinary Settings
  entry restores the session's selected group. No input loss or implicit save.
- Check no navigation key, TOML table or preset directory has been persisted.
- Check cached-file query ownership and background-runtime failure behavior
  survive composition changes. No synchronous service/device discovery.
- Verify actual M1–M5 tests and guard names, affected owner tests and strict
  repository checks. Record V1–V3 as open until the operator supplies evidence.
- If Settings scroll evidence also satisfies part of ADR 0030, record that
  exact subclaim. Do not close remaining Music scroll or unrelated checks.

## Later Phase Review

- Confirm prerequisite completion before implementation, especially ADR 0066's
  shared writer/transition and full configuration-format gate. No implied waiver.
- Inspect the versioned config/preset schema and existing-config fixtures before
  the first write. Reject unsupported versions without partial application.
- Trace metadata selectors to existing service/registry identities. No tokens
  in TOML, preset content, logs, debug output or command arguments.
- Test component coherence, conflicting cross-component combinations, missing
  resources and repeated recall. Reject automatic mask expansion or defaulting.
- Trace explicit apply effects and failed/stale results through the actual
  owners. Saving cannot activate a new chain or retarget an active player.
- Keep Mixxx audio external. Audio configuration requires independent Show and
  audition ownership; the current shared-player path cannot satisfy that proof.

## Review Result Format

For each implemented packet, record:

1. Pass/fail for the reviewed diff and scope.
2. Required fixes, including architectural drift and missing tests.
3. Optional improvements kept outside the packet.
4. Mechanical evidence and remaining operator criteria, each named separately.
5. Whether the implementation can be merged and whether the next packet's
   actual prerequisites are met. Never equate these two decisions.

Update the packet, phase plan, ADR partial status, delivery index and any
runnable pending-human check together. Existing playback gates remain deferred;
this Settings decision provides no evidence that playback has passed.
