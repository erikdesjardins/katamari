use crate::err::Error;
use chrono::{DateTime, Utc};
use http_body_util::{BodyExt, Empty};
use hyper::body::Bytes;
use hyper::header::ACCEPT;
use hyper::{Method, Request, Uri};
use hyper_rustls::HttpsConnector;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use thiserror::Error;

pub type FetchClient = Client<HttpsConnector<HttpConnector>, Empty<Bytes>>;

#[derive(Debug)]
pub struct Feed {
    pub title: String,
    pub logo_url: Option<String>,
}

#[derive(Debug)]
pub struct Entry {
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

pub async fn rss(client: FetchClient, url: Uri) -> Result<(Feed, Vec<Entry>), Error> {
    let request = Request::builder()
        .method(Method::GET)
        .uri(url)
        .header(
            ACCEPT,
            "application/atom+xml, application/rss+xml, application/xml;q=0.9, text/xml;q=0.8",
        )
        .body(Default::default())?;

    let response = client.request(request).await?;

    let rss = response.into_body().collect().await?;

    let raw_feed = feed_rs::parser::parse(&*rss.to_bytes())?;

    let feed = Feed {
        title: raw_feed.title.ok_or(RssError::MissingFeedTitle)?.content,
        logo_url: raw_feed.logo.map(|l| l.uri),
    };

    let entries = raw_feed
        .entries
        .into_iter()
        .map(|item| {
            Ok(Entry {
                timestamp: item
                    .published
                    .or(item.updated)
                    .ok_or(RssError::MissingTimestamp)?,
                href: item
                    .links
                    .into_iter()
                    .next()
                    .ok_or(RssError::MissingLink)?
                    .href,
                title: item.title.ok_or(RssError::MissingTitle)?.content,
                thumbnail_url: item
                    .media
                    .into_iter()
                    .next()
                    .and_then(|m| m.thumbnails.into_iter().next())
                    .map(|t| t.image.uri),
                summary: item.summary.map(|s| s.content),
            })
        })
        .collect::<Result<Vec<_>, RssError>>()?;

    tracing::debug!("parsed feed: {:#?}", feed);
    tracing::debug!("first entry: {:#?}", entries.first());

    Ok((feed, entries))
}
