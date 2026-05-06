# MusicBrainz Picard: Lookup & Scan Internals

Technical reference for how Picard identifies audio files via its **Lookup** (metadata-based) and **Scan** (fingerprint-based) operations.

Source: [metabrainz/picard](https://github.com/metabrainz/picard) on GitHub.

---

## Architecture Overview

```
UI Action (button/shortcut/context menu)
        |
   Tagger dispatcher
        |
   +----+----+
   |         |
Lookup     Scan
   |         |
   v         v
MB API    fpcalc (chromaprint)
search         |
   |      AcoustID API
   |         |
   v         v
 _lookup_finished() callback
        |
  Similarity scoring
        |
  move_file_to_track() / move_file_to_nat()
```

**Key classes:**

| Class | File | Role |
|-------|------|------|
| `Tagger` | `picard/tagger.py` | Application core, dispatches lookup/scan |
| `File` | `picard/file.py` | Per-file lookup logic and match scoring |
| `Cluster` | `picard/cluster.py` | Album-level lookup for grouped files |
| `Album` | `picard/album.py` | Loads release data, matches files to tracks |
| `Metadata` | `picard/metadata.py` | Tag storage + comparison/scoring methods |
| `WebService` | `picard/webservice/__init__.py` | Async HTTP with rate control and queuing |
| `MBAPIHelper` | `picard/webservice/api_helpers/musicbrainz.py` | MusicBrainz REST API wrapper |
| `AcoustIdAPIHelper` | `picard/webservice/api_helpers/acoustid.py` | AcoustID fingerprint API wrapper |
| `AcoustIDClient` | `picard/acoustid/__init__.py` | Fingerprint generation via fpcalc |
| `RecordingResolver` | `picard/acoustid/recordings.py` | Scores and resolves AcoustID results |

---

## 1. Lookup (Ctrl+L)

Metadata-based identification. Uses existing tags (title, artist, album, ISRC, duration) to search the MusicBrainz database.

### 1.1 UI Trigger

**`picard/ui/mainwindow/actions.py`** -- `MainAction.AUTOTAG`

```python
@add_action(MainAction.AUTOTAG)
def _create_autotag_action(parent):
    action = QtGui.QAction(icontheme.lookup('picard-auto-tag'), _("&Lookup"), parent)
    action.setShortcut(QtGui.QKeySequence(_("Ctrl+L")))
    action.triggered.connect(parent.autotag)
```

Also available via context menu in `picard/ui/itemviews/basetreeview.py` and the Tools menu.

### 1.2 Dispatcher

**`picard/tagger.py`** -- `Tagger.autotag()`

```python
def autotag(self, objects):
    for obj in objects:
        if obj.can_autotag:
            obj.lookup_metadata()
```

Polymorphic dispatch -- `File`, `Cluster`, and `Album` each implement `lookup_metadata()` differently.

### 1.3 File Lookup

**`picard/file.py`** -- `File.lookup_metadata()`

Extracts metadata from the file's existing tags and queries the MusicBrainz recording search API:

```python
self._lookup_task = self.tagger.mb_api.find_tracks(
    partial(self._lookup_finished, File.LookupType.METADATA),
    track=metadata['title'],
    artist=metadata['artist'],
    release=metadata['album'],
    tnum=metadata['tracknumber'],
    tracks=metadata['totaltracks'],
    qdur=str(metadata.length // 2000),
    isrc=metadata['isrc'],
    limit=config.setting['query_limit'],
)
```

**API call produced:**

```
GET /ws/2/recording?query=recording:(title) artist:(artist) release:(album) tnum:(n) tracks:(n) qdur:(seconds) isrc:(code)&limit=25
```

Uses Lucene query syntax. Special characters escaped via `escape_lucene_query()`.

### 1.4 Cluster Lookup

**`picard/cluster.py`** -- `Cluster.lookup_metadata()`

Clusters group files by album. Lookup searches for releases instead of recordings:

```python
self._lookup_task = self.tagger.mb_api.find_releases(
    artist=self.metadata['albumartist'],
    release=self.metadata['album'],
    tracks=len(self.files),
    limit=config.setting['query_limit'],
    callback=self._lookup_finished
)
```

**API call produced:**

```
GET /ws/2/release?query=release:(album) artist:(albumartist) tracks:(count)&limit=25
```

Results scored against cluster metadata using `CLUSTER_COMPARISON_WEIGHTS`:

```python
CLUSTER_COMPARISON_WEIGHTS = {
    'album':            17,
    'albumartist':       6,
    'date':              4,
    'format':            2,
    'releasecountry':    2,
    'releasetype':      10,
    'totalalbumtracks':  5,
}
```

### 1.5 Response Handling & Match Scoring

**`picard/file.py`** -- `File._lookup_finished()`

```python
def _lookup_finished(self, lookuptype, document, http, error):
    tracks = document['recordings']
    threshold = config.setting['file_lookup_threshold']
    trackmatch = self._match_to_track(tracks, threshold=threshold)
```

`_match_to_track()` scores each candidate recording using `FILE_COMPARISON_WEIGHTS`:

```python
FILE_COMPARISON_WEIGHTS = {
    'releasetype':      14,
    'title':            13,
    'length':           10,
    'album':             5,
    'totaltracks':       4,
    'artist':            4,
    'date':              4,
    'releasecountry':    2,
    'format':            2,
    'isvideo':           2,
}
```

For each candidate track, every release it appears on is evaluated. Best (release, track) pair wins.

### 1.6 Similarity Algorithms

**`picard/similarity.py`**

Two functions drive text comparison:

- **`similarity(a, b)`** -- Single-word edit-distance comparison. Normalizes to lowercase, strips non-alphanumeric. Returns 0.0-1.0.
- **`similarity2(a, b)`** -- Multi-word comparison. Splits on whitespace/punctuation, greedily matches words (threshold > 0.6 to consume), penalizes unmatched words at 40%.

**`picard/metadata.py`** -- Length scoring:

```python
1.0 - min(abs(a - b), 30000) / 30000.0
```

Identical lengths = 1.0. Difference >= 30 seconds = 0.0. Linear interpolation between.

### 1.7 Result Application

```python
if trackmatch:
    (recording_id, release_group_id, release_id, acoustid, node) = trackmatch
    if release_group_id is not None:
        self.tagger.move_file_to_track(self, release_id, recording_id)
    else:
        self.tagger.move_file_to_nat(self, recording_id)
```

- **`move_file_to_track()`** -- Loads the album via `load_album(release_id)`, sets `file.match_recordingid`, calls `album.match_files([file])` to slot the file into the correct track position.
- **`move_file_to_nat()`** -- Places file as a Non-Album Track when no release association exists.

---

## 2. Scan (Ctrl+Y)

Fingerprint-based identification. Generates an audio fingerprint via Chromaprint/fpcalc, queries AcoustID, then resolves to MusicBrainz recordings.

### 2.1 UI Trigger

**`picard/ui/mainwindow/actions.py`** -- `MainAction.ANALYZE`

```python
@add_action(MainAction.ANALYZE)
def _create_analyze_action(parent):
    action = QtGui.QAction(icontheme.lookup('picard-analyze'), _("S&can"), parent)
    action.setShortcut(QtGui.QKeySequence(_("Ctrl+Y")))
    action.triggered.connect(parent.analyze)
```

Status tip: *"Use AcoustID audio fingerprint to identify the files by the actual music, even if they have no metadata"*

### 2.2 Dispatcher

**`picard/tagger.py`** -- `Tagger.analyze()`

```python
def analyze(self, objs):
    if not self.use_acoustid:
        return
    for file in iter_files_from_objects(objs):
        if file.can_analyze:
            file.set_pending()
            self._acoustid.analyze(file, partial(file._lookup_finished,
                File.LookupType.ACOUSTID))
```

`iter_files_from_objects()` flattens mixed selections (albums, clusters, tracks, files) into a deduplicated file iterator.

### 2.3 Fingerprint Generation

**`picard/acoustid/__init__.py`** -- `AcoustIDClient`

#### Cache check
`analyze()` first checks if the file already has a cached fingerprint in metadata. If so, skips fpcalc and proceeds directly to AcoustID lookup.

#### fpcalc subprocess
If no cached fingerprint:

```
fpcalc -json -length 120 <file_path>
```

- `-json` -- output as JSON
- `-length 120` -- analyze up to 120 seconds of audio
- Runs as a `QProcess` subprocess
- Respects concurrency limits from config

#### Result parsing (`_on_fpcalc_finished`)
- Validates exit code (accepts 0 or 3/DECODING_ERROR)
- Parses JSON output to extract:
  - `fingerprint` -- the Chromaprint fingerprint string
  - `duration` -- audio length in seconds (integer)
- Stores via `file.set_acoustid_fingerprint(fingerprint, length)`

### 2.4 AcoustID API Lookup

**`picard/webservice/api_helpers/acoustid.py`** -- `AcoustIdAPIHelper`

**`picard/acoustid/__init__.py`** -- `_lookup_fingerprint()`

```python
params = {
    'meta': 'recordings releasegroups releases tracks compress sources',
    'fingerprint': fingerprint_string,
    'duration': duration_in_seconds,
}
self._acoustid_api.query_acoustid(callback, **params)
```

**API call produced:**

```
POST https://api.acoustid.org/v2/lookup
Content-Type: application/x-www-form-urlencoded

client=v8pQ6oyB&clientversion=<version>&format=json
&fingerprint=<chromaprint>&duration=<seconds>
&meta=recordings+releasegroups+releases+tracks+compress+sources
```

- Client key: `v8pQ6oyB`
- Rate limit: 333ms minimum between requests

### 2.5 AcoustID Result Processing

**`picard/acoustid/recordings.py`** -- `RecordingResolver`

AcoustID returns a list of possible matches, each with:
- A confidence score (0.0-1.0)
- Associated recordings with source counts

#### Scoring formula:

```
score = min(recording.sources / max_sources, 1.0) * 100 * acoustid_confidence
```

Combines AcoustID confidence with source-count normalization.

#### Metadata resolution:
- Recordings with complete metadata are cached directly
- Incomplete recordings (>= 25% of highest source count) are fetched from MusicBrainz API:
  ```
  GET /ws/2/recording/<id>?inc=artist-credits+release-groups+releases+media
  ```

**`picard/acoustid/json_helpers.py`** -- `parse_recording()` converts AcoustID response format into MusicBrainz-compatible JSON structures.

### 2.6 Match & Result Application

Same callback as Lookup: `File._lookup_finished()` with `LookupType.ACOUSTID`.

Key difference: **threshold = 0** for AcoustID matches (all results considered, vs. configurable threshold for metadata lookup).

```python
trackmatch = self._match_to_track(tracks, threshold=0)
```

On match:
1. Sets `self.metadata['acoustid_id']` with the matched AcoustID
2. Registers with AcoustID manager: `self.tagger.acoustidmanager.add(self, recording_id)`
3. Routes to `move_file_to_track()` or `move_file_to_nat()` (same as Lookup)

---

## 3. Web Service Infrastructure

### 3.1 Request Pipeline

**`picard/webservice/__init__.py`** -- `WebService`

```
API call → RequestPriorityQueue → rate control check → QNetworkAccessManager → response parser → callback
```

- Requests queued by priority and target host
- `_run_next_task()` dequeues and executes, respecting per-host rate limits
- Response parsed by registered handlers (JSON or XML)
- Qt signals track pending request counts

### 3.2 Rate Control

**`picard/webservice/ratecontrol.py`**

TCP-like congestion control per host:
- **Slow start** -- exponential window growth after errors clear
- **Congestion avoidance** -- linear growth at steady state
- Adaptive delay with exponential backoff (max ~30s)
- `get_delay_to_next_request()` enforces minimum intervals
- AcoustID: hardcoded 333ms minimum

### 3.3 Thread Pools

`Tagger` manages three pools:
1. **Main pool** -- general background tasks (min 3 threads, scales dynamically)
2. **Priority pool** -- single-threaded, UI-responsive operations
3. **Save pool** -- single-threaded, prevents file I/O races

---

## 4. Lookup vs. Scan Comparison

| Aspect | Lookup (Ctrl+L) | Scan (Ctrl+Y) |
|--------|-----------------|----------------|
| **Input** | Existing metadata tags | Raw audio signal |
| **External tool** | None | fpcalc (Chromaprint) |
| **Primary API** | MusicBrainz `/ws/2/recording` search | AcoustID `/v2/lookup` |
| **Secondary API** | None | MusicBrainz recording fetch for incomplete results |
| **Query basis** | Title, artist, album, ISRC, duration, track number | Audio fingerprint + duration |
| **Match threshold** | Configurable (`file_lookup_threshold`) | 0 (accept all) |
| **Works without tags** | Poorly -- needs at least title/artist | Yes -- identifies by audio content |
| **Works for clusters** | Yes (`find_releases`) | No -- files only |
| **Scoring weights** | `FILE_COMPARISON_WEIGHTS` (title=13, releasetype=14, length=10) | AcoustID confidence * source count ratio, then same weights |
| **Callback** | `_lookup_finished(LookupType.METADATA)` | `_lookup_finished(LookupType.ACOUSTID)` |
| **Sets acoustid_id** | Only if present in response | Always on match |

---

## 5. Complete Flow Diagrams

### Lookup Flow

```
User selects file(s) → Ctrl+L / Lookup button
    |
Tagger.autotag(objects)
    |
    +-- File: file.lookup_metadata()
    |       |
    |       mb_api.find_tracks(title, artist, album, isrc, duration...)
    |       |
    |       GET /ws/2/recording?query=...
    |       |
    |       _lookup_finished(METADATA, response)
    |       |
    |       _match_to_track(tracks, threshold=file_lookup_threshold)
    |       |  compare each (recording, release) pair
    |       |  using FILE_COMPARISON_WEIGHTS
    |       |
    |       Best match above threshold?
    |       |           |
    |      Yes          No
    |       |           |
    |  move_file_to_track()  "No matching tracks"
    |
    +-- Cluster: cluster.lookup_metadata()
    |       |
    |       mb_api.find_releases(albumartist, album, tracks=count)
    |       |
    |       GET /ws/2/release?query=...
    |       |
    |       _lookup_finished → _match_to_release()
    |       using CLUSTER_COMPARISON_WEIGHTS
    |       |
    |       load_album(best_release_id)
    |
    +-- Album: album.load() → refresh from MB API
```

### Scan Flow

```
User selects file(s) → Ctrl+Y / Scan button
    |
Tagger.analyze(objects)
    |
    +-- use_acoustid enabled?
    |       No → return
    |
    iter_files_from_objects(objects)
    |
    For each file:
        |
        file.set_pending()
        |
        AcoustIDClient.analyze(file, callback)
            |
            +-- Cached fingerprint? → skip to lookup
            |
            _fingerprint(file) → queue task
            |
            _run_next_task()
            |
            QProcess: fpcalc -json -length 120 <path>
            |
            _on_fpcalc_finished()
            |  parse JSON → extract fingerprint + duration
            |
            file.set_acoustid_fingerprint(fp, length)
            |
            _lookup_fingerprint()
            |
            POST https://api.acoustid.org/v2/lookup
            |  fingerprint, duration, meta=recordings+releases+...
            |
            RecordingResolver processes results
            |  score = (sources/max_sources) * confidence * 100
            |
            +-- Complete metadata? → cache
            +-- Incomplete (>=25% sources)? → GET /ws/2/recording/<id>
            |
            file._lookup_finished(ACOUSTID, results)
            |
            _match_to_track(tracks, threshold=0)
            |
            Sets metadata['acoustid_id']
            Registers with acoustidmanager
            |
            move_file_to_track() or move_file_to_nat()
```
