# ADR 0076: Playlist RSS Check For Stale MusicIndex Records

## Status

Accepted - 2026-09-24. The operator gave each decision below on 2026-09-23 and accepted this ADR on 2026-09-24.
Implementation partial: packets 001 to 006 and ADR 0075 packet 020 are implemented on 2026-09-24 and 2026-09-25 with mechanical checks Green. Their visual gates are open and paused.
The operator reviewed each decision on 2026-09-24. The review changed Decisions 3, 5 and 7.

Amended 2026-09-25: the operator limited the `Retry-After` wait to 60 seconds. This tightens Decision 2. "Accepted Values" records the limit.
"Accepted Values" records the three numeric values that the operator accepted on 2026-09-24.

## Context

ADR 0075 made the app select a source for each field each time it showed that field.
Each field then needed a source order, a freshness window, expiry, stale labels, and removal rules.
ADR 0075 collected more than one hundred accepted refinements for that selection.

ADR 0075 Decision I then made MusicIndex a cache of RSS and RSS the only provenance.
The selection rules stayed. They conflict with Decision I when the app fetched a feed and its RSS observation is no longer fresh.
The operator rejected that direction on 2026-09-23.

These facts were found on 2026-09-23:

- The local Stophammer database, dated 2026-04-19, holds 7,632 feeds. Wavlake hosts 6,652 of them.
- The local podping archive holds 23,877 feeds and no Wavlake feed. The archive can be partial.
  If Wavlake sends no podpings, MusicIndex finds a Wavlake change only when Stophammer crawls that feed again.
- Wavlake throttles crawlers. The Stophammer Wavlake import waits a minimum of 2 seconds between requests and backs off for 300 seconds after HTTP `429`.
- Wavlake serves feeds through a CDN with an `ETag` and `cache-control: max-age=43200`.
  A conditional request returned `304` from the CDN cache.

A periodic RSS check of each Library feed would send requests to Wavlake from each installed app.
The operator chose a check that runs only when a show needs current values.

## Decision

### 1. MusicIndex Is The Default And The Normal Checker

The app stores one current value for each field. MusicIndex supplies the first value.
The app selects no source when it shows a field. It shows the stored value.

The existing "check for updates" control continues to ask MusicIndex.
The app has no periodic RSS check.

Ownership stays as ADR 0075 states it: the channel, the item, or a person.
A feed value never becomes a track value.

### 2. A Playlist Check Reads RSS

Each playlist has a "Check RSS" button.
The same check runs automatically when the operator selects a playlist for a show.

The check reads the RSS document of each feed that has a track in the playlist. It reads each feed once.
It sends conditional GET requests. It sends one request at a time to each host, with a minimum interval between them.
It obeys `Retry-After`. After HTTP `429`, it sends no more requests to that host during this check, and the report names the feeds that it did not check.

The check runs as a runtime actor under ADR 0040. The mounted view updates in place.
A failed request or a failed parse changes no stored value.

### 3. The Check Compares Each Direct Element And The Item List

A difference in any element that RSS states directly makes the MusicIndex record stale.
These elements are titles, descriptions, artwork, links, the audio URL, duration, dates, the explicit flag, and language.
They also include artist text, `podcast:person` credits, the Nostr `podcast:txt` value, and `podcast:value` payment routes.

A track that RSS adds or removes also makes the MusicIndex record stale.

The check also compares the album's own `<podcast:publisher>` remote item, its `feedGuid` and its `feedUrl`.
The album states this element. ADR 0077 treats it as RSS. A difference updates the stored `music_to_publisher` relationship of that album.
The operator added this element on 2026-09-24.

MusicIndex computes some values that RSS does not state directly.
These are the publisher link state, the publisher role, a release date from the oldest item, and counts.
The check does not compare them. The app keeps the MusicIndex value for them.

Each compared element uses a written comparison rule. Descriptions compare as readable text.
URLs use the accepted normalization of scheme, host, and default ports.
Formatting alone does not make a record stale.

### 4. The App Applies RSS Values Automatically And Reports Them

When the check finds a difference, the app writes the RSS value to the stored field at once.
A field that RSS removes after a successful parse is cleared.

The app then shows one report for each check.
For each difference, the report names the feed, the field, the old value, the new value, and the time of the check.
The report links to podping.me for each stale feed. The app sends no podping.

The observation store keeps the original MusicIndex and RSS responses.
An automatic update therefore keeps the earlier value as evidence.

### 5. An RSS Value Holds Until MusicIndex Agrees

After an RSS value replaces a MusicIndex value, a later MusicIndex fetch does not overwrite that field.
The field returns to normal MusicIndex updates when MusicIndex supplies the same value as the last RSS check.
It also returns when MusicIndex supplies a record that it updated after the last RSS check.
Without this rule, each MusicIndex fetch would write the stale value back.

The operator added the second condition on 2026-09-24. It covers an RSS change that MusicIndex receives before the next playlist check.
The packet for this rule confirms that the MusicIndex `updated_at` value records the ingest of the RSS document. If it does not, the packet stops and reports.

### 6. A New Track Is Stored And Offered For Download

When RSS contains a track that the app does not have, the app adds that track to the stored tracks of the feed.
It downloads nothing. The report lists the track with the existing download action.

### 7. A Removed Track Is Marked And Kept

When RSS no longer contains a stored track, the app marks the track "removed from feed" with the time of the check.
The app keeps the track, its file, its playlist entries, and its cue entries.
The report lists the track. The mark clears if the track returns to RSS.

A track with this mark in a show playlist is not ready for the show.
The artist removed it, so its payment route can be incorrect.
The operator removes the track from the playlist, or confirms that the show plays it. The operator added this rule on 2026-09-24.

### 8. Audio Tags Change Only On Operator Confirmation

The check changes the database only. It writes no audio tag.
ADR 0075 §7 keeps this rule: a metadata refresh does not write audio tags as a side effect.

When the tags of Library files differ from the stored metadata, the app shows an "Update n file(s)" button.
The button opens a popup that lists each affected file and has a confirm button.
The confirm button writes the tags of all listed files. Tag writes use the existing tag boundary of ADRs 0004 and 0008.

The app does not write a file that a show plays or holds in its cue.
The popup marks that file "in use by the show", and the app writes the other files.
The button keeps the count of the files that it did not write. A later confirmation writes them after the show releases them.

### 9. Payment Routes In Files Come From The Database

Each write of a payment route to a file uses the route stored in the database.
The route repair of ADR 0065 asks MusicIndex only when the database has no route for that track.
It never replaces a route that the RSS check set.

A playlist track whose file route differs from the stored route is not ready for a show.
The track stays not ready until the operator confirms "Update n file(s)".
The operator therefore cannot go live with a known wrong payment route without a warning.

## Accepted Values

The operator accepted each value below on 2026-09-24.

| Detail | Value |
|---|---|
| Minimum interval between requests to one host | 2 seconds, the same as the Stophammer Wavlake import |
| Parallel hosts | At most four hosts at the same time |
| Stored validators | The app stores the `ETag` and `Last-Modified` values of each feed with its RSS observation |
| Maximum `Retry-After` wait | 60 seconds. A longer value stops that host for the check, as HTTP `429` does. The report names the host and the requested wait. Accepted on 2026-09-25 |

## Out Of Scope

Stophammer removed its public artist credits on 2026-04-08, in commit `a16a720`.
Since then, the artist binding of ADR 0045 (`src/identity_ingest.rs`) receives no artist identifier.
ADR 0077 replaces that binding. It is not part of this decision.

The deployed API also sends no `Feed.name`, `Track.name`, or `Track.feed_url`.
The app deletes the code that reads each field when its removal is confirmed.

## Relationship To Other Decisions

This ADR supersedes these parts of ADR 0075 from 2026-09-24:

- Decisions F, G, and H.
- The Decision I rule that fresh RSS always supplies a value.
- The Decision I rule that MusicIndex supplies a value only for a feed that the app has not fetched.
- The provider priority, freshness, expiry, and stale-label parts of each accepted field refinement.

These parts of ADR 0075 stay in force:

- The Decision I definition of provenance, and the podping.me direction without an outbound podping.
- Decisions B, C, D, and E.
- The extraction orders, placeholder rules, date precision and display rules, duration formats, URL action rules, and fallback sections in the field refinements.
- Sections 1, 2, 3, 4, 5, and 7.

This ADR also amends two decisions from 2026-09-24:

- ADR 0065. The route repair uses the stored route and asks MusicIndex only when no stored route exists.
- ADR 0059. A difference between the file route and the stored route makes a track not ready. A track with the "removed from feed" mark is also not ready until the operator confirms it or removes it.

The acceptance change of 2026-09-24 did these items. The [phase plan](../plans/adr-0076-playlist-rss-check-phase-plan.md) registers the packets, written on 2026-09-24:

- ADRs 0075, 0065, and 0059 record the change in their Status sections.
- The ADR index, `AGENTS.md`, and the ADR 0075 phase plan change.
- Packet 045 is replaced by packets for the playlist check, the comparison and report, the tag update button, and the readiness rule.
- Packet 020 no longer selects a source. It projects the stored values with their owners.
- Guards that assert superseded rules are deleted.

## Verification

Mechanical criteria, phrased at the owning layer:

- The playlist check requests each distinct feed of the playlist once.
- The check sends one request at a time to each host, with the accepted minimum interval. A test with an injected clock proves this.
- After HTTP `429`, the check sends no more requests to that host, and the result names each unchecked feed.
- The check sends a conditional request when a stored validator exists, and treats `304` as no change.
- The comparison reports no difference for two descriptions with the same readable text.
- The comparison reports a difference for each compared element, and no difference for a derived field.
- An RSS difference writes the stored field, and a later different MusicIndex value does not overwrite it.
- A later equal MusicIndex value returns the field to normal MusicIndex updates.
- A failed RSS request changes no stored value.
- A new RSS track creates a stored track with no file. A missing RSS track sets the removed mark and deletes nothing.
- The readiness view model reports a track with the removed mark as not ready, until the operator confirms it or removes it from the playlist.
- A changed `<podcast:publisher>` remote item updates the stored `music_to_publisher` relationship. A changed link state or role that MusicIndex derives causes no difference.
- A MusicIndex record updated after the last RSS check returns a held field to normal MusicIndex updates.
- The check writes no audio tag.
- The tag update skips a file that the show plays or holds in its cue, and keeps it in the count.
- The route repair uses a stored route and does not replace a route that the RSS check set.
- The readiness view model reports a track as not ready when its file route differs from the stored route.
- The view model of the report exposes the feed, field, old value, new value, check time, and podping.me link for each difference.

Visual criteria, for an operator check after the visual pause ends:

- The report is readable in Light and Dark themes at normal and narrow widths.
- The "Check RSS" button, the "Update n file(s)" button, and its popup show the correct state, count, and file list.
- The "removed from feed" mark, its not-ready state and its confirmation are clear on the playlist and on the Show surface.
- A view that is open during a check updates in place.

## Alternatives Considered

- Keep the ADR 0075 selection rules. The operator rejected this on 2026-09-23. The rules conflict with Decision I, and each new field adds more rules.
- A daily RSS check of each Library feed. Each installed app would send requests to Wavlake, which throttles crawlers.
- A daily recrawl in Stophammer. It keeps MusicIndex current for all clients, but it needs a decision in that repository. This ADR does not need it.
- Fall back to MusicIndex when an RSS observation expires. Displayed values would change after a restart with no new data.
- Write audio tags automatically. That changes audio files in the background, possibly during a live show.

## Consequences

The app shows one stored value for each field and selects no source at display time.
Most per-field source rules in ADR 0075 are no longer needed.
The app sends RSS requests only for the feeds of a playlist that a show uses, or on the operator's request.
Search and feed views outside a show keep the MusicIndex lag for feeds that send no podping.
File tags can lag behind the stored metadata until the operator confirms the tag update.
A show warns about each track whose file carries an outdated payment route.
