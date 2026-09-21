-- ADR 0075 packet 012: frozen synthetic version-11 input.
-- Source revision: a12521e7510b3d05cd4fc097a370f1f965145aad; registry unchanged at capture.
-- Captured 2026-09-20 by adr_0075_migration_export_version_11.
-- Complete registry execution stopped at migration 11 AfterApply before ledger insertion.
-- Every legacy table has synthetic records. Never initialize an app database from this file.
BEGIN TRANSACTION;
CREATE TABLE artist_source_facts (
            id INTEGER PRIMARY KEY,
            source TEXT NOT NULL CHECK (source != ''),
            source_artist_id TEXT NOT NULL CHECK (source_artist_id != ''),
            name TEXT NULL,
            sort_name TEXT NULL,
            image_url TEXT NULL,
            website_url TEXT NULL,
            aliases_json TEXT NOT NULL DEFAULT '[]',
            tags_json TEXT NOT NULL DEFAULT '[]',
            area TEXT NULL,
            begin_year INTEGER NULL,
            end_year INTEGER NULL,
            observed_at INTEGER NULL,
            raw_json TEXT NULL,
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(source, source_artist_id)
        );
INSERT INTO "artist_source_facts" VALUES(91,'musicindex','artist-91','Frozen artist',NULL,NULL,NULL,'["Alias"]','[]',NULL,NULL,NULL,NULL,NULL,'2026-09-20 14:01:23');
CREATE TABLE artist_source_ids (
            id INTEGER PRIMARY KEY,
            artist_source_fact_id INTEGER NOT NULL
                REFERENCES artist_source_facts(id) ON DELETE CASCADE,
            entity_type TEXT NULL,
            entity_id TEXT NULL,
            position INTEGER NULL,
            scheme TEXT NULL,
            value TEXT NULL,
            extraction_path TEXT NULL,
            observed_at INTEGER NULL,
            raw_json TEXT NULL,
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
INSERT INTO "artist_source_ids" VALUES(93,91,NULL,NULL,NULL,'musicbrainz','synthetic-id',NULL,NULL,NULL,'2026-09-20 14:01:23');
CREATE TABLE artist_source_links (
            id INTEGER PRIMARY KEY,
            artist_source_fact_id INTEGER NOT NULL
                REFERENCES artist_source_facts(id) ON DELETE CASCADE,
            entity_type TEXT NULL,
            entity_id TEXT NULL,
            position INTEGER NULL,
            link_type TEXT NULL,
            url TEXT NULL,
            extraction_path TEXT NULL,
            observed_at INTEGER NULL,
            raw_json TEXT NULL,
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
INSERT INTO "artist_source_links" VALUES(92,91,NULL,NULL,NULL,'website','https://fixture.invalid/artist',NULL,NULL,NULL,'2026-09-20 14:01:23');
CREATE TABLE broadcast_event_selection (
            singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
            event_id TEXT NOT NULL CHECK (event_id != ''),
            revision INTEGER NOT NULL CHECK (revision > 0)
        );
INSERT INTO "broadcast_event_selection" VALUES(1,'frozen-event',7);
CREATE TABLE broadcast_events (
            id INTEGER PRIMARY KEY,
            event_id TEXT NOT NULL UNIQUE CHECK (event_id != ''),
            label TEXT NULL,
            endpoint TEXT NOT NULL CHECK (endpoint != ''),
            token_path TEXT NOT NULL CHECK (token_path != ''),
            created_at INTEGER NOT NULL,
            last_checked_at INTEGER NULL,
            last_status TEXT NULL CHECK (
                last_status IS NULL
                    OR last_status IN ('unknown', 'live', 'dead')
            )
        );
INSERT INTO "broadcast_events" VALUES(101,'frozen-event','Frozen show','http://127.0.0.1:9','/fixture/secret-file',123,NULL,'unknown');
CREATE TABLE entity_contributors (
            id INTEGER PRIMARY KEY,
            owner_kind TEXT NOT NULL,
            feed_id INTEGER NULL REFERENCES feeds(id) ON DELETE CASCADE,
            track_id INTEGER NULL REFERENCES tracks(id) ON DELETE CASCADE,
            position INTEGER NOT NULL,
            name TEXT NULL,
            role TEXT NULL,
            group_name TEXT NULL,
            href TEXT NULL,
            image_url TEXT NULL,
            nostr_npub TEXT NULL,
            source TEXT NOT NULL CHECK (source != ''),
            raw_json TEXT NULL,
            observed_at INTEGER NULL,
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            CHECK (
                (owner_kind = 'feed'
                    AND feed_id IS NOT NULL
                    AND track_id IS NULL)
                OR (owner_kind = 'track'
                    AND feed_id IS NULL
                    AND track_id IS NOT NULL)
            )
        );
INSERT INTO "entity_contributors" VALUES(83,'feed',41,NULL,4,'Frozen contributor','producer',NULL,NULL,NULL,NULL,'rss','{"entity_type":"feed","entity_id":"feed-41","position":4,"source":"publisher","extraction_path":"rss/person","observed_at":52,"role_norm":"producer"}',NULL,'2026-09-20 14:01:23');
CREATE TABLE entity_identity_ids (
            id INTEGER PRIMARY KEY,
            owner_kind TEXT NOT NULL,
            feed_id INTEGER NULL REFERENCES feeds(id) ON DELETE CASCADE,
            track_id INTEGER NULL REFERENCES tracks(id) ON DELETE CASCADE,
            contributor_position INTEGER NULL,
            entity_type TEXT NULL,
            entity_id TEXT NULL,
            position INTEGER NULL,
            scheme TEXT NULL,
            value TEXT NULL,
            source TEXT NOT NULL CHECK (source != ''),
            extraction_path TEXT NULL,
            observed_at INTEGER NULL,
            raw_json TEXT NULL,
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            CHECK (
                (owner_kind = 'feed'
                    AND feed_id IS NOT NULL
                    AND track_id IS NULL
                    AND contributor_position IS NULL)
                OR (owner_kind = 'track'
                    AND feed_id IS NULL
                    AND track_id IS NOT NULL
                    AND contributor_position IS NULL)
                OR (owner_kind = 'feed_contributor'
                    AND feed_id IS NOT NULL
                    AND track_id IS NULL
                    AND contributor_position IS NOT NULL)
                OR (owner_kind = 'track_contributor'
                    AND feed_id IS NULL
                    AND track_id IS NOT NULL
                    AND contributor_position IS NOT NULL)
            )
        );
INSERT INTO "entity_identity_ids" VALUES(82,'track',NULL,51,NULL,NULL,NULL,NULL,'unsupported','retained-value','rss',NULL,NULL,'{"owner":null}','2026-09-20 14:01:23');
CREATE TABLE entity_identity_links (
            id INTEGER PRIMARY KEY,
            owner_kind TEXT NOT NULL,
            feed_id INTEGER NULL REFERENCES feeds(id) ON DELETE CASCADE,
            track_id INTEGER NULL REFERENCES tracks(id) ON DELETE CASCADE,
            contributor_position INTEGER NULL,
            entity_type TEXT NULL,
            entity_id TEXT NULL,
            position INTEGER NULL,
            link_type TEXT NULL,
            url TEXT NULL,
            source TEXT NOT NULL CHECK (source != ''),
            extraction_path TEXT NULL,
            observed_at INTEGER NULL,
            raw_json TEXT NULL,
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            CHECK (
                (owner_kind = 'feed'
                    AND feed_id IS NOT NULL
                    AND track_id IS NULL
                    AND contributor_position IS NULL)
                OR (owner_kind = 'track'
                    AND feed_id IS NULL
                    AND track_id IS NOT NULL
                    AND contributor_position IS NULL)
                OR (owner_kind = 'feed_contributor'
                    AND feed_id IS NOT NULL
                    AND track_id IS NULL
                    AND contributor_position IS NOT NULL)
                OR (owner_kind = 'track_contributor'
                    AND feed_id IS NULL
                    AND track_id IS NOT NULL
                    AND contributor_position IS NOT NULL)
            )
        );
INSERT INTO "entity_identity_links" VALUES(81,'feed',41,NULL,NULL,'track','contradictory-owner',3,'website','https://fixture.invalid/claim','rss',NULL,NULL,'{"unresolved":true}','2026-09-20 14:01:23');
CREATE TABLE entity_metadata_facts (
            id INTEGER PRIMARY KEY,
            owner_kind TEXT NOT NULL,
            feed_id INTEGER NULL REFERENCES feeds(id) ON DELETE CASCADE,
            track_id INTEGER NULL REFERENCES tracks(id) ON DELETE CASCADE,
            fact_key TEXT NOT NULL CHECK (fact_key != ''),
            value_text TEXT NULL,
            value_integer INTEGER NULL,
            value_boolean INTEGER NULL CHECK (value_boolean IN (0, 1)),
            source TEXT NOT NULL CHECK (source != ''),
            extraction_path TEXT NULL,
            observed_at INTEGER NULL,
            raw_json TEXT NULL,
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            CHECK (
                (owner_kind = 'feed'
                    AND feed_id IS NOT NULL
                    AND track_id IS NULL)
                OR (owner_kind = 'track'
                    AND feed_id IS NULL
                    AND track_id IS NOT NULL)
            ),
            CHECK (
                (value_text IS NOT NULL)
                + (value_integer IS NOT NULL)
                + (value_boolean IS NOT NULL) = 1
            )
        );
INSERT INTO "entity_metadata_facts" VALUES(84,'track',NULL,51,'description','Retained claim',NULL,NULL,'musicindex',NULL,NULL,'{"unknown":"retained"}','2026-09-20 14:01:23');
CREATE TABLE feeds (
            id INTEGER PRIMARY KEY,
            feed_url TEXT NOT NULL UNIQUE,
            feed_guid TEXT NULL,              -- podcast:guid if present
            title TEXT NULL,
            link TEXT NULL,
            language TEXT NULL,
            description TEXT NULL,
            podcast_medium TEXT NULL,
            album_image_href TEXT NULL,
            album_image_mime TEXT NULL,
            people_json TEXT NULL,
            podcast_value_json TEXT NULL,
            is_subscribed INTEGER NOT NULL DEFAULT 0,
            last_fetched_at TEXT NOT NULL DEFAULT (datetime('now')),
            extra_json TEXT NOT NULL DEFAULT '{}'
        , musicindex_updated_at INTEGER);
INSERT INTO "feeds" VALUES(41,'https://fixture.invalid/feed','feed-41','Frozen feed',NULL,NULL,'Legacy description',NULL,NULL,NULL,NULL,NULL,0,'2026-09-20 14:01:23','{"enclosures":[{"url":"https://fixture.invalid/song","source":"rss","entity_type":"feed","entity_id":"feed-41","position":2}],"transcripts":[{"url":"https://fixture.invalid/text","language":"fr"}]}',NULL);
CREATE TABLE local_files (
            id INTEGER PRIMARY KEY,
            path TEXT NOT NULL UNIQUE,
            track_id INTEGER NULL REFERENCES tracks(id) ON DELETE SET NULL,
            added_at TEXT NOT NULL DEFAULT (datetime('now')),
            file_size_bytes INTEGER NULL,
            audio_duration_sec INTEGER NULL,
            checksum TEXT NULL,
            extra_json TEXT NOT NULL DEFAULT '{}'
        );
INSERT INTO "local_files" VALUES(61,'Frozen/track.flac',51,'2026-09-20 14:01:23',17,NULL,'synthetic-checksum','{}');
CREATE TABLE local_path_repairs (
            id INTEGER PRIMARY KEY,
            track_id INTEGER NULL REFERENCES tracks(id) ON DELETE SET NULL,
            old_path TEXT NOT NULL UNIQUE CHECK (old_path != ''),
            reason TEXT NOT NULL CHECK (reason != ''),
            recorded_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
INSERT INTO "local_path_repairs" VALUES(111,52,'/old/unresolved.flac','missing','2026-09-20 14:01:23');
CREATE TABLE playback_sessions (
            session_id TEXT PRIMARY KEY,
            sequence INTEGER NOT NULL DEFAULT 0,
            local_track_id INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
            playlist_id INTEGER NULL REFERENCES playlists(id) ON DELETE SET NULL,
            playlist_position INTEGER NULL,
            started_at TEXT NOT NULL,
            position_ms INTEGER NOT NULL DEFAULT 0,
            state TEXT NOT NULL DEFAULT 'stopped',
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
INSERT INTO "playback_sessions" VALUES('frozen-session',9,51,71,1,'2026-09-20T00:00:00Z',2345,'paused','2026-09-20 14:01:23');
CREATE TABLE playlist_tracks (
            playlist_id INTEGER NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
            track_id    INTEGER NOT NULL REFERENCES tracks(id)    ON DELETE CASCADE,
            position    INTEGER NOT NULL,
            PRIMARY KEY (playlist_id, position)
        );
INSERT INTO "playlist_tracks" VALUES(71,52,0);
INSERT INTO "playlist_tracks" VALUES(71,51,1);
CREATE TABLE playlists (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            description TEXT,
            created_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
            updated_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
        );
INSERT INTO "playlists" VALUES(71,'Frozen playlist','Keep order',1,2);
CREATE TABLE schema_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
INSERT INTO "schema_migrations" VALUES(1,'feeds_musicindex_updated_at','2026-09-20 14:00:13');
INSERT INTO "schema_migrations" VALUES(2,'tracks_enclosure_type','2026-09-20 14:00:13');
INSERT INTO "schema_migrations" VALUES(3,'identity_source_facts','2026-09-20 14:00:13');
INSERT INTO "schema_migrations" VALUES(4,'artist_source_facts','2026-09-20 14:00:13');
INSERT INTO "schema_migrations" VALUES(5,'track_artist_source_bindings','2026-09-20 14:00:13');
INSERT INTO "schema_migrations" VALUES(6,'cleanup_placeholder_source_text','2026-09-20 14:00:13');
INSERT INTO "schema_migrations" VALUES(7,'cleanup_markup_placeholder_source_text','2026-09-20 14:00:13');
INSERT INTO "schema_migrations" VALUES(8,'metadata_source_facts','2026-09-20 14:00:13');
INSERT INTO "schema_migrations" VALUES(9,'broadcast_events','2026-09-20 14:00:13');
INSERT INTO "schema_migrations" VALUES(10,'local_path_repairs','2026-09-20 14:00:13');
CREATE TABLE schema_version (
            version INTEGER NOT NULL
        );
INSERT INTO "schema_version" VALUES(1);
CREATE TABLE track_artist_source_bindings (
            id INTEGER PRIMARY KEY,
            track_id INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
            role TEXT NOT NULL CHECK (role != ''),
            source TEXT NOT NULL CHECK (source != ''),
            source_artist_id TEXT NOT NULL CHECK (source_artist_id != ''),
            confidence REAL NULL,
            provenance TEXT NULL,
            observed_at INTEGER NULL,
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(track_id, role, source, source_artist_id),
            FOREIGN KEY (source, source_artist_id)
                REFERENCES artist_source_facts(source, source_artist_id)
                ON DELETE CASCADE
        );
INSERT INTO "track_artist_source_bindings" VALUES(94,51,'performer','musicindex','artist-91',0.5,NULL,NULL,'2026-09-20 14:01:23');
CREATE TABLE tracks (
            id INTEGER PRIMARY KEY,
            feed_id INTEGER NOT NULL REFERENCES feeds(id) ON DELETE CASCADE,
            item_guid TEXT NOT NULL,
            enclosure_url TEXT NULL,
            enclosure_type TEXT NULL,
            link TEXT NULL,
            pub_date TEXT NULL,
            track_title TEXT NULL,
            artist_name TEXT NULL,
            album_title TEXT NULL,
            album_artist_name TEXT NULL,
            disc_number INTEGER NULL,
            track_number INTEGER NULL,
            duration_seconds INTEGER NULL,
            itunes_duration_raw TEXT NULL,
            itunes_explicit TEXT NULL,
            track_image_href TEXT NULL,
            track_image_mime TEXT NULL,
            people_json TEXT NULL,
            item_value_json TEXT NULL,
            is_in_library INTEGER NOT NULL DEFAULT 0,
            extra_json TEXT NOT NULL DEFAULT '{}',
            UNIQUE(feed_id, item_guid)
        );
INSERT INTO "tracks" VALUES(51,41,'track-51',NULL,NULL,NULL,NULL,'Frozen track',NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,1,'{"enclosures":[{"source":"rss","entity_type":"track","entity_id":"track-51","extraction_path":"rss/item/enclosure","observed_at":42}]}');
INSERT INTO "tracks" VALUES(52,41,'track-52',NULL,NULL,NULL,NULL,'Second track',NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,NULL,1,'{}');
CREATE INDEX idx_feeds_guid ON feeds(feed_guid);
CREATE INDEX idx_feeds_is_subscribed ON feeds(is_subscribed);
CREATE INDEX idx_tracks_feed_id       ON tracks(feed_id);
CREATE INDEX idx_tracks_track_number  ON tracks(feed_id, track_number);
CREATE INDEX idx_tracks_is_in_library ON tracks(is_in_library);
CREATE INDEX idx_local_files_track_id ON local_files(track_id);
CREATE INDEX idx_playlist_tracks_track_id ON playlist_tracks(track_id);
CREATE INDEX idx_playback_sessions_track_id
            ON playback_sessions(local_track_id);
CREATE INDEX idx_entity_identity_links_owner
            ON entity_identity_links(owner_kind, feed_id, track_id, contributor_position);
CREATE INDEX idx_entity_identity_links_owner_source
            ON entity_identity_links(owner_kind, feed_id, track_id, contributor_position, source);
CREATE INDEX idx_entity_identity_links_feed_id
            ON entity_identity_links(feed_id);
CREATE INDEX idx_entity_identity_links_track_id
            ON entity_identity_links(track_id);
CREATE INDEX idx_entity_identity_ids_owner
            ON entity_identity_ids(owner_kind, feed_id, track_id, contributor_position);
CREATE INDEX idx_entity_identity_ids_owner_source
            ON entity_identity_ids(owner_kind, feed_id, track_id, contributor_position, source);
CREATE INDEX idx_entity_identity_ids_feed_id
            ON entity_identity_ids(feed_id);
CREATE INDEX idx_entity_identity_ids_track_id
            ON entity_identity_ids(track_id);
CREATE INDEX idx_entity_contributors_owner
            ON entity_contributors(owner_kind, feed_id, track_id);
CREATE INDEX idx_entity_contributors_owner_source
            ON entity_contributors(owner_kind, feed_id, track_id, source);
CREATE INDEX idx_entity_contributors_feed_id
            ON entity_contributors(feed_id);
CREATE INDEX idx_entity_contributors_track_id
            ON entity_contributors(track_id);
CREATE INDEX idx_artist_source_facts_source_artist
            ON artist_source_facts(source, source_artist_id);
CREATE INDEX idx_artist_source_links_fact
            ON artist_source_links(artist_source_fact_id);
CREATE INDEX idx_artist_source_ids_fact
            ON artist_source_ids(artist_source_fact_id);
CREATE INDEX idx_track_artist_bindings_track
            ON track_artist_source_bindings(track_id);
CREATE INDEX idx_track_artist_bindings_source_artist
            ON track_artist_source_bindings(source, source_artist_id);
CREATE INDEX idx_track_artist_bindings_role
            ON track_artist_source_bindings(role);
CREATE INDEX idx_entity_metadata_facts_owner
            ON entity_metadata_facts(owner_kind, feed_id, track_id);
CREATE INDEX idx_entity_metadata_facts_owner_source
            ON entity_metadata_facts(owner_kind, feed_id, track_id, source);
CREATE INDEX idx_entity_metadata_facts_feed_id
            ON entity_metadata_facts(feed_id);
CREATE INDEX idx_entity_metadata_facts_track_id
            ON entity_metadata_facts(track_id);
CREATE INDEX idx_broadcast_events_created_at
            ON broadcast_events(created_at);
CREATE INDEX idx_broadcast_events_status
            ON broadcast_events(last_status);
CREATE INDEX idx_local_path_repairs_track_id
            ON local_path_repairs(track_id);
COMMIT;
