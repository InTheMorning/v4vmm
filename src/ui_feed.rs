use crate::api::{Feed, Track};
use crate::search::{
    fmt_date, fmt_runtime, render_action_row, render_collapsed_text_section, render_feed_header,
    render_track_list_section, InspectorFrame, SearchApp,
};
use crate::ui_common::{optional_row, render_detail_grid};
use crate::ui_context::ViewContext;
use crate::views::FeedView;
use gpui::{div, prelude::*, AnyElement, Context};
use std::collections::BTreeMap;

pub(crate) fn render_feed_view(
    view: &FeedView,
    tracks: &[Track],
    _ctx: &ViewContext,
    frame: &InspectorFrame,
    app: &mut SearchApp,
    cx: &mut Context<SearchApp>,
) -> AnyElement {
    use crate::ui::theme::spacing;

    let title = view.title.clone().unwrap_or_else(|| "Unknown Feed".into());
    let artist = view.artist.clone().unwrap_or_else(|| "Unknown".into());

    let mut rows = vec![
        (
            "Release Kind".to_string(),
            view.release_kind
                .clone()
                .unwrap_or_else(|| "Unknown".into()),
        ),
        (
            "Publisher".to_string(),
            view.publisher_text
                .clone()
                .unwrap_or_else(|| "Unknown".into()),
        ),
    ];
    optional_row(
        &mut rows,
        "Release Date",
        view.release_date.and_then(fmt_date),
    );
    optional_row(&mut rows, "Language", view.language.clone());
    if view.explicit == Some(true) {
        rows.push(("Explicit".into(), "Yes".into()));
    }
    optional_row(
        &mut rows,
        "Tracks",
        view.episode_count.map(|n| n.to_string()),
    );

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

    div()
        .flex()
        .flex_col()
        .gap(spacing::LG)
        .child(render_feed_header(
            frame,
            &header_feed,
            &title,
            Some(artist.as_str()),
            cx,
        ))
        .child(render_action_row(frame, &BTreeMap::new(), app, cx))
        .child(render_detail_grid(rows))
        .when(view.description.is_some(), |el| {
            el.child(render_collapsed_text_section(
                "Description",
                view.description.clone().unwrap_or_default(),
            ))
        })
        .when(!sorted_tracks.is_empty(), |el| {
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
            el.child(render_track_list_section(
                "Tracks",
                format!(
                    "{} total{}",
                    sorted_tracks.len(),
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
            ))
        })
        .into_any_element()
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
