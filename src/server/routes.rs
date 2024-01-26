use crate::err::Error;
use crate::fetch::{self, Item};
use crate::server::AppState;
use axum::extract::{RawQuery, State};
use axum::response::{Html, IntoResponse};
use chrono::{Local, NaiveDate};
use std::cmp;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
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
pub async fn index(
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
    let mut all_items = Vec::new();
    while let Some(result) = pending_feeds.join_next().await {
        let (_feed, items) = result??;
        all_items.extend(items);
    }

    // Collect items into one list per day, and deduplicate them.

    all_items.sort_by_key(|item| cmp::Reverse(item.timestamp));

    struct Day {
        date: NaiveDate,
        items_with_count: Vec<(Item, usize)>,
        url_prefix_to_index: HashMap<String, usize>,
    }

    let mut days = Vec::<Day>::new();

    for item in all_items {
        // Check whether we need to start a new day.
        let date = item.timestamp.with_timezone(&Local).date_naive();
        if days.last().map(|d| d.date) != Some(date) {
            days.push(Day {
                date,
                items_with_count: Default::default(),
                url_prefix_to_index: Default::default(),
            });
        }
        // Get or insert this item.
        let day = days.last_mut().unwrap();
        // Strip hash from the URL, so multiple links to the same page (but e.g. for different events with different anchors) are deduplicated.
        let url_prefix = item
            .href
            .split_once('#')
            .map(|(prefix, _)| prefix)
            .unwrap_or(&item.href);
        match day.url_prefix_to_index.entry(url_prefix.to_owned()) {
            // If we've already seen this item, increment its count,
            // and override the entry (so the oldest entry is used).
            Entry::Occupied(entry) => {
                let index = *entry.get();
                day.items_with_count[index].0 = item;
                day.items_with_count[index].1 += 1;
            }
            // Otherwise, add a new item.
            Entry::Vacant(entry) => {
                let index = day.items_with_count.len();
                day.items_with_count.push((item, 1));
                entry.insert(index);
            }
        }
    }

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

    for day in days {
        html.push_str(&format!("<h1>{}</h1>", day.date));

        for (item, count) in day.items_with_count {
            let item_count = if count > 1 {
                format!(" ({}x)", count)
            } else {
                String::new()
            };
            let summary = if let Some(summary) = item.summary {
                format!("<br/><sup>╚ {}</sup>", summary)
            } else {
                String::new()
            };
            html.push_str(&format!(
                r#"<li><a href="{}">{}</a>{}{}</li>"#,
                item.href, item.title, item_count, summary
            ));
        }
    }

    Ok(Html(html))
}
