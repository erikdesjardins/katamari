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

pub struct Entry {
    pub timestamp: DateTime<Utc>,
    pub href: String,
    pub title: String,
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
}

pub async fn rss(client: FetchClient, url: Uri) -> Result<Vec<Entry>, Error> {
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

    let feed = feed_rs::parser::parse(&*rss.to_bytes())?;

    let entries = feed
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
                summary: item.summary.map(|s| s.content),
            })
        })
        .collect::<Result<_, RssError>>()?;

    Ok(entries)
}
