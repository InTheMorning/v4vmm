//! Metadata command family.

use crate::application::command_bus::{ApplicationCommand, CommandOutcome, CommandResult};
use crate::application::command_context::CommandContext;
use crate::application::errors::command::CommandError;
use crate::application::events::metadata::MetadataEvent;
use crate::application::events::ApplicationEvent;
use crate::db::TrackRow;
use crate::feed_service::{self, StagedMusicBrainzLookup};
use crate::metadata::MusicBrainzLookupResult;
use crate::musicbrainz::{lookup_releases, LookupMetadata, MusicBrainzCandidate};

use reqwest::blocking::Client as ReqwestClient;

/// Looks up `MusicBrainz` candidates for one local library track.
#[derive(Clone, Debug)]
pub struct LookupMusicBrainzTrack {
    track: TrackRow,
}

impl LookupMusicBrainzTrack {
    /// Creates a track `MusicBrainz` lookup command.
    #[must_use]
    pub const fn new(track: TrackRow) -> Self {
        Self { track }
    }
}

impl ApplicationCommand for LookupMusicBrainzTrack {
    type Output = MusicBrainzLookupResult;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let result = feed_service::lookup_musicbrainz_library_track(&self.track)
            .map_err(|error| metadata_command_error(&error))?;
        Ok(CommandOutcome::without_events(result))
    }
}

/// Looks up `MusicBrainz` release candidates for album-level staging.
#[derive(Clone, Debug)]
pub struct LookupMusicBrainzAlbumReleases {
    metadata: LookupMetadata,
    limit: i32,
}

impl LookupMusicBrainzAlbumReleases {
    /// Creates an album release lookup command.
    #[must_use]
    pub const fn new(metadata: LookupMetadata, limit: i32) -> Self {
        Self { metadata, limit }
    }
}

impl ApplicationCommand for LookupMusicBrainzAlbumReleases {
    type Output = Vec<MusicBrainzCandidate>;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let musicbrainz_client = ReqwestClient::builder()
            .user_agent(format!(
                "v4vmm/{} (MusicBrainz metadata lookup)",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|error| CommandError::Metadata(format!("{error:#}")))?;
        let candidates = lookup_releases(&musicbrainz_client, &self.metadata, self.limit)
            .map_err(|error| metadata_command_error(&error))?;
        Ok(CommandOutcome::without_events(candidates))
    }
}

/// Stages `MusicBrainz` metadata for one local library track.
#[derive(Clone, Debug)]
pub struct StageMusicBrainzTrack {
    track: TrackRow,
}

impl StageMusicBrainzTrack {
    /// Creates a track `MusicBrainz` staging command.
    #[must_use]
    pub const fn new(track: TrackRow) -> Self {
        Self { track }
    }
}

impl ApplicationCommand for StageMusicBrainzTrack {
    type Output = StagedMusicBrainzLookup;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let staged = feed_service::lookup_musicbrainz_stage_for_track(&self.track)
            .map_err(|error| metadata_command_error(&error))?;
        Ok(CommandOutcome::new(
            staged,
            metadata_track_tagged_events(self.track.id),
        ))
    }
}

/// Stages one chosen `MusicBrainz` candidate for one local library track.
#[derive(Clone, Debug)]
pub struct StageMusicBrainzCandidate {
    track: TrackRow,
    candidate: MusicBrainzCandidate,
}

impl StageMusicBrainzCandidate {
    /// Creates a selected-candidate `MusicBrainz` staging command.
    #[must_use]
    pub const fn new(track: TrackRow, candidate: MusicBrainzCandidate) -> Self {
        Self { track, candidate }
    }
}

impl ApplicationCommand for StageMusicBrainzCandidate {
    type Output = StagedMusicBrainzLookup;

    fn execute(self, context: &CommandContext) -> CommandResult<Self::Output> {
        if context.cancellation().is_cancelled() {
            return Err(CommandError::Cancelled);
        }
        let staged = feed_service::stage_candidate_for_track(&self.track, &self.candidate)
            .map_err(|error| metadata_command_error(&error))?;
        Ok(CommandOutcome::new(
            staged,
            metadata_track_tagged_events(self.track.id),
        ))
    }
}

fn metadata_track_tagged_events(track_id: i64) -> Vec<ApplicationEvent> {
    vec![ApplicationEvent::Metadata(MetadataEvent::TrackTagged {
        track_id,
    })]
}

fn metadata_command_error(error: &anyhow::Error) -> CommandError {
    CommandError::Metadata(format!("{error:#}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::application::command_bus::CommandBus;
    use crate::application::command_context::{CancellationToken, OperationId, TraceId};

    fn cancelled_context() -> CommandContext {
        let cancellation = CancellationToken::new();
        cancellation.cancel();
        CommandContext::new(OperationId::new(1), cancellation, TraceId::new(1))
    }

    fn track_without_local_file() -> TrackRow {
        TrackRow {
            id: 1,
            feed_id: 2,
            feed_guid: Some("feed-guid".into()),
            item_guid: "track-guid".into(),
            track_title: Some("Track".into()),
            artist_name: Some("Artist".into()),
            album_title: Some("Album".into()),
            album_artist_name: Some("Artist".into()),
            track_number: Some(1),
            disc_number: None,
            duration_seconds: None,
            enclosure_url: None,
            enclosure_type: None,
            track_image_href: None,
            is_in_library: true,
            feed_title: Some("Feed".into()),
            album_image_href: None,
            local_path: None,
            transcript_url: None,
        }
    }

    #[test]
    fn lookup_musicbrainz_track_honors_cancelled_context() {
        let error = CommandBus::new()
            .execute(
                LookupMusicBrainzTrack::new(track_without_local_file()),
                &cancelled_context(),
            )
            .expect_err("cancelled lookup should fail before service call");

        assert_eq!(error, CommandError::Cancelled);
    }

    #[test]
    fn lookup_musicbrainz_album_releases_honors_cancelled_context() {
        let error = CommandBus::new()
            .execute(
                LookupMusicBrainzAlbumReleases::new(LookupMetadata::default(), 3),
                &cancelled_context(),
            )
            .expect_err("cancelled album lookup should fail before network call");

        assert_eq!(error, CommandError::Cancelled);
    }

    #[test]
    fn stage_musicbrainz_track_honors_cancelled_context() {
        let error = CommandBus::new()
            .execute(
                StageMusicBrainzTrack::new(track_without_local_file()),
                &cancelled_context(),
            )
            .expect_err("cancelled staging should fail before service call");

        assert_eq!(error, CommandError::Cancelled);
    }

    #[test]
    fn stage_musicbrainz_candidate_honors_cancelled_context() {
        let error = CommandBus::new()
            .execute(
                StageMusicBrainzCandidate::new(
                    track_without_local_file(),
                    MusicBrainzCandidate {
                        recording_id: "recording".into(),
                        title: "Track".into(),
                        ..MusicBrainzCandidate::default()
                    },
                ),
                &cancelled_context(),
            )
            .expect_err("cancelled candidate staging should fail before service call");

        assert_eq!(error, CommandError::Cancelled);
    }
}
