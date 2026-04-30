# Problems

~~Discover back button not present once a feed/track/etc is clicked.~~
~~Need RSS feed icon link~~
~~Need audio url icon link~~
~~Fix search ui bogus "load more" button duplicates results.~~
~~Fix artist result inspector view:  show only source feeds for that artist's tracks.  Clicking feed drills down to show tracks in with proper ordering.~~
~~RSS icon should also be shown in library view feeds, not just Discover.~~
~~Feed links shown in track view should have feed title and feed guid should show on hover only.~~

~~Detect errors when attempting to read/write id3~~
~~downloads file extension according to mimetype in enclosure~~
~~Typing in search bar hides the "recents" page forever.~~

~~Fix animated gifs~~

Improve download:
 - rename from subscribe/unsubscribe
 - show status

figure out what to do with podcast links in the podroll:
  - If PI API token set: API call to podcast index to fetch basic info 
    - tag as podcast with a watermark podcast icon over albumart

# Features
~~Support podroll~~
~~Add nostr link icon~~
~~Add Flac and ogg support~~
~~Add artist search~~

Show MusicL playlists

Add playlist support
  - drag to reorder instead of arrows
  - exportable to musicL, m3u (music_dir/playlists)


Make a service architecture with consistent DAO (data access object) interface.
  - Allow for showing the same UI layout for the same metadata, regardless of whether it is musicindex online metadata, or local file metadata.
  - Reuse the same Discover Artist and Feed views in Library.
    - The only difference between library view and discover view should be at the track level:
      - Show the compare and/or musicbrainz buttons only in Library views.

Show download/remove button on each track when viewing a feed.
Show add/remove to/from playlist button on each track when viewing a feed.
Make feed-level download/remove indicate downloaded only when entire feed is fully downloaded:
  - manually removing one track from the feed should toggle entire-feed-downloaded state to false (downloadable)
  - manually downloading all tracks in a feed should toggle entire-feed-downloaded state to true (removable)


# Test
Alternate enclosures support
  - support for video metadata
    - ftyp = what flavor of file this is
    - moov = table of contents / structure / timing / track map
    - mdat = the actual encoded bytes
    - meta/keys/ilst = human-style metadata like title, artist, custom fields
      - MP4/MOV file
        └─ moov
           └─ meta
              ├─ keys
              │  ├─ key[1] = "title"
              │  ├─ key[2] = "artist"
              │  ├─ key[3] = "RSS_FEED_GUID"
              │  ├─ key[4] = "RSS_TRACK_GUID"
              │  └─ key[5] = "VALUE_URL"
              └─ ilst
                 ├─ item(type=1)
                 │  └─ data = "My Song"
                 ├─ item(type=2)
                 │  └─ data = "Citizen"
                 ├─ item(type=3)
                 │  └─ data = "feed-guid-123"
                 ├─ item(type=4)
                 │  └─ data = "track-guid-456"
                 └─ item(type=5)
                    └─ data = "https://example.com/split"

# Uploader tool

Design a tool for feed-writers to tag their music before upload.

- Allow for uploading tagged music to a cloud provider/vps via rsync, ssh, etc.
- Autogenerate corresponding rss feed
  - Allow for separate upload method for the feed

- Allow to save a artist and album profiles to have new tracks inherit some default id3 values