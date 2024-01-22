use crate::err::Error;
use crate::fetch;
use crate::server::AppState;
use axum::extract::{RawQuery, State};
use axum::http::uri::InvalidUri;
use axum::response::{Html, IntoResponse};
use std::cmp;
use std::sync::Arc;
use thiserror::Error;
use tokio::spawn;

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
    let pending_feeds = params
        .split('&')
        .map(|url| {
            let url = url.parse()?;
            let feed = spawn(fetch::rss(state.client.clone(), url));
            Ok(feed)
        })
        .collect::<Result<Vec<_>, InvalidUri>>()?;

    // ...and wait for them to finish.
    let mut entries = Vec::new();
    for pending_feed in pending_feeds {
        entries.extend(pending_feed.await??);
    }

    entries.sort_by_key(|entry| cmp::Reverse(entry.timestamp));

    Ok(Html(format!(
        r#"
        <!DOCTYPE html>
        <html>
            <head>
                <meta charset="utf-8">
            </head>
            <body>
                <ul>
                    {}
                </ul>
            </body>
        </html>
        "#,
        entries
            .into_iter()
            .map(|item| { format!(r#"<li><a href="{}">{}</a></li>"#, item.href, item.title) })
            .collect::<Vec<_>>()
            .join("\n")
    )))
}
