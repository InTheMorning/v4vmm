use crate::api::{Feed, Track};
use crate::search::{
    detail_rows_from_strings, fmt_date, fmt_runtime, render_action_row,
    render_collapsed_text_section, render_feed_header, render_publisher_link_value,
    render_track_list_section, InspectorFrame, SearchApp,
};
use crate::ui::composites::DetailGrid;
use crate::ui::primitives::VStack;
use crate::ui::tokens::Spacing;
use crate::ui_common::{optional_row, DetailRow};
use crate::ui_context::ViewContext;
use crate::views::FeedView;
use gpui::{prelude::*, AnyElement, Context};
use std::collections::BTreeMap;

pub(crate) fn render_feed_view(
    view: &FeedView,
    tracks: &[Track],
    _ctx: &ViewContext,
    frame: &InspectorFrame,
    app: &mut SearchApp,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    let title = view.title.clone().unwrap_or_else(|| "Unknown Feed".into());
    let artist = view.artist.clone().unwrap_or_else(|| "Unknown".into());

    let mut scalar_rows = vec![(
        "Release Kind".to_string(),
        view.release_kind
            .clone()
            .unwrap_or_else(|| "Unknown".into()),
    )];
    optional_row(
        &mut scalar_rows,
        "Release Date",
        view.release_date.and_then(fmt_date),
    );
    optional_row(&mut scalar_rows, "Language", view.language.clone());
    if view.explicit == Some(true) {
        scalar_rows.push(("Explicit".into(), "Yes".into()));
    }
    optional_row(
        &mut scalar_rows,
        "Tracks",
        view.episode_count.map(|n| n.to_string()),
    );
    let mut rows = detail_rows_from_strings(scalar_rows);
    if let Some(publisher) = view
        .publisher_text
        .as_deref()
        .map(str::trim)
        .filter(|publisher| !publisher.is_empty())
    {
        rows.insert(
            1,
            DetailRow {
                key: "Publisher".into(),
                value: render_publisher_link_value(publisher.to_string(), cx),
            },
        );
    } else {
        let mut publisher_rows =
            detail_rows_from_strings(vec![("Publisher".into(), "Unknown".into())]);
        rows.insert(1, publisher_rows.remove(0));
    }

    let mut sorted_tracks = tracks.to_vec();
    sorted_tracks.sort_by(|a, b| {
        let a_num = a.track_number.unwrap_or(i32::MAX);
        let b_num = b.track_number.unwrap_or(i32::MAX);
        a_num.cmp(&b_num).then_with(|| {
            b.pub_date
                .unwrap_or_default()
                .cmp(&a.pub_date.unwrap_or_default())
        })
    });
    let total_secs: i32 = sorted_tracks.iter().filter_map(|t| t.duration_secs).sum();

    let header_feed = feed_view_to_api(view);

    let mut stack = VStack::new()
        .spacing(Spacing::LG)
        .stretch()
        .child(render_feed_header(
            frame,
            &header_feed,
            &title,
            Some(artist.as_str()),
            cx,
        ))
        .child(render_action_row(frame, &BTreeMap::new(), app, cx))
        .child(DetailGrid::new(
            rows.into_iter().map(Into::into).collect::<Vec<_>>(),
        ));

    if let Some(description) = view.description.clone() {
        stack = stack.child(render_collapsed_text_section("Description", description));
    }

    if !sorted_tracks.is_empty() {
        let playlists = app.playlists.clone();
        let open_guid = frame.add_to_playlist_open_track_guid.clone();
        let feed_guid = frame.entity_id.clone();
        let feed_url = view.feed_url.clone();
        let feed_context = Some((
            feed_guid.as_str(),
            feed_url.as_deref(),
            open_guid.as_deref(),
            playlists.as_slice(),
        ));
        let feed_for_tracks = if view.feed_guid.is_some() || view.feed_url.is_some() {
            Some(Feed {
                feed_guid: view.feed_guid.clone(),
                feed_url: view.feed_url.clone(),
                title: view.title.clone(),
                ..Default::default()
            })
        } else {
            None
        };
        let count = sorted_tracks.len();
        stack = stack.child(render_track_list_section(
            "Tracks",
            format!(
                "{} total{}",
                count,
                if total_secs > 0 {
                    format!(" · {}", fmt_runtime(total_secs))
                } else {
                    String::new()
                }
            ),
            sorted_tracks,
            feed_for_tracks,
            feed_context,
            app,
            cx,
        ));
    }

    stack.into_any_element()
}

fn feed_view_to_api(view: &FeedView) -> Feed {
    Feed {
        feed_guid: view.feed_guid.clone(),
        feed_url: view.feed_url.clone(),
        title: view.title.clone(),
        name: view.title.clone(),
        release_artist: view.artist.clone(),
        image_url: view.image_url.clone(),
        release_date: view.release_date,
        language: view.language.clone(),
        explicit: view.explicit,
        episode_count: view.episode_count,
        release_kind: view.release_kind.clone(),
        publisher_text: view.publisher_text.clone(),
        description: view.description.clone(),
        payment_routes: Some(view.payment_routes.clone()),
        source_contributors: Some(view.contributors.clone()),
        ..Feed::default()
    }
}
