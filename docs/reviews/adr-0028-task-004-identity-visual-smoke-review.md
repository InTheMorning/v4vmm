# ADR 0028 Task 004 Identity Visual Smoke Review

## Result

Pass with follow-up noted - 2026-05-01.

## Scope

- Copied the user's config, thumbnail cache, and SQLite database into
  `/tmp/v4vmm-adr28-identity-smoke` so the smoke pass did not write to the real
  library.
- Launched `target/debug/v4vmm` with isolated XDG config/data directories.
- Compared `The Heycitizen Experience` in Library and Discover at the same
  1676 x 1008 viewport.
- Seeded only the copied database with `adr-0028-smoke-fixture` source facts:
  one feed website link, one feed Nostr id, and one feed contributor row.
- Captured screenshots locally:
  - `/tmp/v4vmm-adr28-library-release-final.png`
  - `/tmp/v4vmm-adr28-discover-release-final.png`

## Findings

| Area | Observation | Result |
|---|---|---|
| Local feed identity hydration | Library renders `Website` and `Nostr` actions from persisted local `entity_identity_links` and `entity_identity_ids` rows. | Pass |
| Discover identity parity | Discover renders the same Website/Nostr action vocabulary from the API-backed projection path. | Pass |
| Shared styling | Both surfaces use the shared identity-action composite. The Nostr action includes the Nostr icon plus text, so the state is not color-only. | Pass |
| Contributor fixture | The copied DB contains a persisted contributor row, and the local projection tests cover hydration. Library release detail does not yet have a contributor panel slot, so this is not visible in the screenshot. | Follow-up |
| Visual parity outside identity facts | Library and Discover still differ in metadata density and action placement. Those differences predate ADR 0028 and are not source-fact persistence failures. | Deferred |
| Architecture | Shared projections remain GPUI-free and database-free. Library loads identity facts while building the screen-owned album snapshot, then renders from pure data. | Pass |

## Verification

Passed:

```bash
cargo fmt -- --check
cargo check
cargo test identity_actions_are_shared_across_surface_contexts
cargo test sources::tests::local_source_fetch_feed_hydrates_feed_and_track_identity_facts
cargo test views::tests::from_local_feed_hydrates_identity_facts_and_contributors
cargo test --test architecture_tests
cargo clippy --lib --tests -- -D warnings
cargo test
```

## Follow-Up

- Add a bounded contributor panel slot for Library release detail if we want
  persisted local contributor identity facts to be visually inspectable without
  opening a Discover lazy panel.
- Keep non-identity Library/Discover visual differences under the shared
  projection/action-state follow-up track rather than expanding ADR 0028.

## Merge Recommendation

Mergeable.
