use crate::err::{Error, ResponseError};
use crate::fetch::{self, Feed, Item};
use crate::server::AppState;
use crate::url;
use axum::extract::{RawQuery, State};
use axum::response::{Html, IntoResponse};
use base64::prelude::BASE64_URL_SAFE;
use base64::Engine;
use chrono::{Local, NaiveDate};
use hyper::Uri;
use std::cmp;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::task::JoinSet;

#[derive(Debug, Error)]
enum GetError {
    #[error("no URLs provided in query string")]
    NoUrls,
    #[error("failed to fetch {0}")]
    FailedToFetch(Uri, #[source] Error),
}

/// Load a list of RSS feeds, provided as query params, and display the results in chronological order.
///
/// e.g. `http://localhost:3000/?https://www.rust-lang.org/feeds/releases.xml&https://blog.rust-lang.org/feed.xml`
pub async fn index(
    State(state): State<Arc<AppState>>,
    RawQuery(params): RawQuery,
) -> Result<impl IntoResponse, ResponseError> {
    let Some(params) = params else {
        return Err(GetError::NoUrls.into());
    };

    // Start all the requests concurrently...
    let mut pending_feeds = JoinSet::new();
    for url in params.split('&') {
        let url: Uri = url.parse()?;
        let feed = fetch::rss(state.client.clone(), url.clone());
        pending_feeds.spawn(async { (url, feed.await) });
    }

    // ...and wait for them to finish.
    let mut all_feeds = Vec::new();
    while let Some(result) = pending_feeds.join_next().await {
        let (url, result) = result?;
        let (feed, items) = result.map_err(|e| GetError::FailedToFetch(url, e))?;
        all_feeds.push((feed, items));
    }

    // Collect all items into one vec, sorted by date.
    let mut all_items = Vec::new();
    for (feed, items) in &mut all_feeds {
        // Carry along a reference to each item's feed.
        all_items.extend(items.drain(..).map(|item| (&*feed, item)));
    }
    all_items.sort_by_key(|(_, item)| cmp::Reverse(item.timestamp));

    // Drop items that point to a prefix of the feed URL.
    all_items.retain(|(feed, item)| {
        let drop = feed.url.starts_with(url::prefix(&item.href));
        if drop {
            // This happens for some unimportant GitHub events, e.g. branch deletion,
            // which just point to the GitHub homepage.
            tracing::debug!("dropping item: {:#?}", item);
        }
        !drop
    });

    // Split items into one vec per day, and deduplicate them.

    struct Day<'a> {
        date: NaiveDate,
        items: Vec<ItemsWithFeed<'a>>,
    }

    struct ItemsWithFeed<'a> {
        feed: &'a Feed,
        item: Item,
        count: usize,
        highlighted: bool,
    }

    let mut days = Vec::<Day>::new();

    {
        let mut url_prefix_to_index = HashMap::<String, usize>::new();
        for (feed, item) in all_items {
            // Check whether we need to start a new day.
            let date = item.timestamp.with_timezone(&Local).date_naive();
            if days.last().map(|d| d.date) != Some(date) {
                days.push(Day {
                    date,
                    items: Default::default(),
                });
                url_prefix_to_index.clear();
            }
            // Get or insert this item.
            let day = days.last_mut().unwrap();
            // Strip hash from the URL, so multiple links to the same page (but e.g. for different events with different anchors) are deduplicated.
            let url_prefix = url::prefix(&item.href);
            match url_prefix_to_index.entry(url_prefix.to_owned()) {
                // If we've already seen this item, increment its count,
                // and override the entry (so the oldest entry is used).
                Entry::Occupied(entry) => {
                    let index = *entry.get();
                    day.items[index].item = item;
                    day.items[index].count += 1;
                }
                // Otherwise, add a new item.
                Entry::Vacant(entry) => {
                    let index = day.items.len();
                    day.items.push(ItemsWithFeed {
                        feed,
                        item,
                        count: 1,
                        highlighted: false,
                    });
                    entry.insert(index);
                }
            }
        }
    }

    // Highlight unique domains for the day.
    {
        let mut domain_to_entry_count = HashMap::<String, usize>::new();
        for day in &mut days {
            domain_to_entry_count.clear();

            // Collect counts by domain.
            for i in &mut day.items {
                let domain = url::domain(&i.item.href);
                *domain_to_entry_count.entry(domain.to_owned()).or_default() += 1;
            }

            let max_count = domain_to_entry_count.values().max().copied().unwrap_or(0);

            // Mark the items with the lowest count as highlighted.
            for i in &mut day.items {
                let domain = url::domain(&i.item.href);
                let entry_count = domain_to_entry_count[domain];
                // Highlight if there are fewer than 3 entries with this domain, and there are less entries than the most common domain.
                i.highlighted = entry_count < 3 && entry_count < max_count;
            }
        }
    }

    let nonce = BASE64_URL_SAFE.encode(rand::random::<[u8; 16]>());

    let mut html = format!(
        r#"
        <!DOCTYPE html>
        <html>
            <head>
                <meta charset="utf-8">
                <meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'nonce-{nonce}'; img-src https://*;" />
                <style nonce="{nonce}">
                    img {{
                        height: 1rem;
                        width: 1rem;
                        vertical-align: middle;
                    }}
                    .spacer {{
                        margin-left: 1rem;
                    }}
                    .highlight {{
                        background-color: aquamarine;
                    }}
                    a:visited {{
                        color: color-mix(in lch, rgb(85, 26, 139), #fff)
                    }}
                </style>
            </head>
            <body>
                <ul>
    "#,
    );

    for day in days {
        html.push_str(&format!("<h1>{}</h1>", day.date));

        for i in day.items {
            let (thumbnail, spacer) = if let Some(thumbnail_url) =
                i.item.thumbnail_url.as_ref().or(i.feed.logo_url.as_ref())
            {
                (
                    format!(
                        r#"<img src="{}" title="{}"/> "#,
                        thumbnail_url, i.feed.title
                    ),
                    r#"<span class="spacer">&nbsp;</spacer>"#,
                )
            } else {
                Default::default()
            };
            let item_count = if i.count > 1 {
                format!(" ({}x)", i.count)
            } else {
                String::new()
            };
            let summary = if let Some(summary) = i.item.summary {
                format!(r#"<br/>{}<sup>└ {}</sup>"#, spacer, summary)
            } else {
                String::new()
            };
            html.push_str(&format!(
                r#"<li>{}<a class="{}" href="{}">{}</a>{}{}</li>"#,
                thumbnail,
                if i.highlighted { "highlight" } else { "" },
                i.item.href,
                i.item.title,
                item_count,
                summary
            ));
        }
    }

    Ok(Html(html))
}
