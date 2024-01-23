use crate::err::Error;
use crate::fetch;
use crate::server::AppState;
use axum::extract::{RawQuery, State};
use axum::response::{Html, IntoResponse};
use chrono::Local;
use std::cmp;
use std::sync::Arc;
use thiserror::Error;
use tokio::task::JoinSet;

#[derive(Debug, Error)]
enum GetError {
    #[error("No URLs provided in query string")]
    NoUrls,
}

/// Load a list of RSS feeds, provided as query params, and display the results in chronological order.
///
/// e.g. `http://localhost:3000/?https://www.rust-lang.org/feeds/releases.xml&https://blog.rust-lang.org/feed.xml`
pub async fn get(
    State(state): State<Arc<AppState>>,
    RawQuery(params): RawQuery,
) -> Result<impl IntoResponse, Error> {
    let Some(params) = params else {
        return Err(GetError::NoUrls.into());
    };

    // Start all the requests concurrently...
    let mut pending_feeds = JoinSet::new();
    for url in params.split('&') {
        let url = url.parse()?;
        let feed = fetch::rss(state.client.clone(), url);
        pending_feeds.spawn(feed);
    }

    // ...and wait for them to finish.
    let mut all_entries = Vec::new();
    while let Some(result) = pending_feeds.join_next().await {
        let (_feed, entries) = result??;
        all_entries.extend(entries);
    }

    all_entries.sort_by_key(|entry| cmp::Reverse(entry.timestamp));

    let mut html = String::from(
        r#"
        <!DOCTYPE html>
        <html>
            <head>
                <meta charset="utf-8">
            </head>
            <body>
                <ul>
    "#,
    );

    let mut last_date = None;
    for entry in all_entries {
        let date = entry.timestamp.with_timezone(&Local).date_naive();
        if last_date != Some(date) {
            html.push_str(&format!("<h1>{}</h1>", date));
            last_date = Some(date);
        }

        html.push_str(&format!(
            r#"<li><a href="{}">{}</a></li>"#,
            entry.href, entry.title
        ));
    }

    Ok(Html(html))
}
