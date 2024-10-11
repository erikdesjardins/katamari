use crate::err::Error;
use crate::url;
use chrono::{DateTime, Utc};
use http_body_util::{BodyExt, Empty};
use hyper::body::Bytes;
use hyper::header::{ACCEPT, USER_AGENT};
use hyper::{Method, Request, Uri};
use hyper_rustls::HttpsConnector;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use thiserror::Error;

mod extract;

pub type FetchClient = Client<HttpsConnector<HttpConnector>, Empty<Bytes>>;

#[derive(Debug)]
pub struct Feed {
    pub url: String,
    pub title: String,
    pub logo_url: Option<String>,
}

#[derive(Debug)]
pub struct Item {
    pub timestamp: DateTime<Utc>,
    pub href: String,
    pub title: String,
    pub thumbnail_url: Option<String>,
    pub summary: Option<String>,
}

#[derive(Debug, Error)]
enum RssError {
    #[error("Missing timestamp")]
    MissingTimestamp,
    #[error("Missing link")]
    MissingLink,
    #[error("Missing title")]
    MissingTitle,
    #[error("Missing feed title")]
    MissingFeedTitle,
}

pub async fn rss(client: FetchClient, url: Uri) -> Result<(Feed, Vec<Item>), Error> {
    let request = Request::builder()
        .method(Method::GET)
        .uri(&url)
        .header(
            ACCEPT,
            "application/atom+xml, application/rss+xml, application/xml;q=0.9, text/xml;q=0.8",
        )
        .header(
            USER_AGENT,
            concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION")),
        )
        .body(Default::default())?;

    let response = client.request(request).await?;

    let rss = response.into_body().collect().await?.to_bytes();

    let raw_feed = feed_rs::parser::parse(&*rss)?;

    let feed = Feed {
        url: url.to_string(),
        title: raw_feed.title.ok_or(RssError::MissingFeedTitle)?.content,
        logo_url: raw_feed.logo.map(|l| l.uri),
    };

    let items = raw_feed
        .entries
        .into_iter()
        .map(|item| {
            let timestamp = item
                .published
                .or(item.updated)
                .ok_or(RssError::MissingTimestamp)?;
            let href = item
                .links
                .into_iter()
                .next()
                .ok_or(RssError::MissingLink)?
                .href;
            let title = item.title.ok_or(RssError::MissingTitle)?.content;
            let thumbnail_url = item
                .media
                .into_iter()
                .next()
                .and_then(|m| m.thumbnails.into_iter().next())
                .map(|t| t.image.uri);
            let summary = extract::summary(&href, item.summary, item.content)?;

            Ok(Item {
                timestamp,
                href: url::make_absolute(&feed.url, href),
                title,
                thumbnail_url: thumbnail_url.map(|t| url::make_absolute(&feed.url, t)),
                summary,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;

    tracing::debug!("parsed feed: {:#?}", feed);
    tracing::debug!("first item: {:#?}", items.first());

    Ok((feed, items))
}
