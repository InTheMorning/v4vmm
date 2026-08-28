# ADR 0010: Configurable MusicIndex Endpoint

## Status

Accepted - 2026-04-21.

## Context

The desktop UI calls MusicIndex for discovery, inspector hydration, contributor/value-route loading, and tag comparison. Operators need to point those calls at the current production API or a compatible alternate endpoint without rebuilding the application.

## Decision

Store a `musicindex_endpoint` key in the existing TOML config. The default is `https://api.musicindex.org`; bare hostnames entered in the UI are normalized to HTTPS URLs. The GPUI shell adds a Settings tab that saves this key and updates the running search view immediately.

All MusicIndex API client construction in the search UI uses the configured endpoint, including follow-up requests made from inspector actions.

## Consequences

- Existing config files continue to load; missing `musicindex_endpoint` falls back to the default.
- Search state is cleared when the endpoint changes so stale results from another API are not mixed with new requests.
- The endpoint is validated as an HTTP or HTTPS URL before it is persisted.
