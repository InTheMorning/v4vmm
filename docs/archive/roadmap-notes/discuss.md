Phase 1: playlist engine + M3U/musicL
Phase 2: mpv IPC adapter
Phase 3: VTS/metadata emitter tied to PlaybackSession
Phase 4: MPRIS adapter
Phase 5: optional internal player

Clementine playing?  v4vmm emits VTS.
mpv playing?         v4vmm emits VTS.
internal player?     same VTS engine.
headless mode?       same VTS engine.

player/audio source
    ↓
now-playing identity + elapsed time
    ↓
VTS resolver
    ↓
remoteValue / socket.io / websocket / HTTP JSON


BAD:
StreamTitle = "Artist - Title {feedGuid}{itemGuid}"

BETTER:
PlaybackSession {
  track_id,
  feed_guid,
  item_guid,
  local_file_id,
  source_url,
  position_ms,
}


v4vmm-core
  library
  playlists
  playback_session
  vts_resolver
  metadata_resolver

v4vmm-player-adapters
  m3u launcher
  mpv IPC
  MPRIS
  liquidsoap/icecast monitor
  internal player later

v4vmm-relay
  websocket/socket.io/http
  remoteValue emitter
  now-playing endpoint



v4vmm local app
  = library/player/session brain

musicindex.org / stophammer
  = catalog + metadata lookup brain

v4vmm-live endpoint
  = public realtime state relay

Icecast/Liquidsoap
  = audio pipe only