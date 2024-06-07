use crate::err::Error;
use feed_rs::model::{Content, Text};
use mediatype::names::{HTML, TEXT};
use mediatype::MediaType;
use quick_xml::events::attributes::Attribute;
use quick_xml::events::Event;
use quick_xml::Reader;
use std::borrow::Cow;
use std::str;

#[cfg(test)]
mod tests;

const TEXT_HTML: MediaType = MediaType::new(TEXT, HTML);

/// Extract a summary, given an item's href and summary/content.
pub fn summary(
    href: &str,
    summary: Option<Text>,
    content: Option<Content>,
) -> Result<Option<String>, Error> {
    // If summary is present...
    if let Some(summary) = summary {
        if summary.content_type.essence() == TEXT_HTML {
            // ...use heuristics to extract summary from HTML.
            return summary_from_html_summary(&summary.content);
        } else {
            // ...assume plaintext summary.
            return Ok(Some(summary.content));
        }
    }

    // If content is present...
    if let Some(content) = content {
        if let Some(body) = content.body {
            if content.content_type.essence() == TEXT_HTML {
                // ...use heuristics to extract content from HTML.
                return summary_from_html_body(href, &body);
            } else {
                // ...assume plaintext content.
                return Ok(Some(body));
            }
        }
    }

    Ok(None)
}

fn summary_from_html_summary(summary: &str) -> Result<Option<String>, Error> {
    let mut reader = Reader::from_str(summary);
    reader.trim_text(true);
    // HTML doesn't require self-closing tags to be formatted properly,
    // e.g. you can do <a><img></a>.
    reader.check_end_names(false);

    loop {
        match reader.read_event()? {
            Event::Eof => {
                break;
            }
            Event::Text(text) => {
                // Return the first text content we find.
                return Ok(Some(text.unescape()?.into_owned()));
            }
            _ => {}
        }
    }

    Ok(None)
}

fn summary_from_html_body(item_href: &str, body: &str) -> Result<Option<String>, Error> {
    let mut reader = Reader::from_str(body);
    reader.trim_text(true);
    // HTML doesn't require self-closing tags to be formatted properly,
    // e.g. you can do <a><img></a>.
    reader.check_end_names(false);

    loop {
        match reader.read_event()? {
            Event::Eof => {
                break;
            }
            Event::Start(tag) if tag.name().as_ref() == b"a" => {
                // If this is a link to the item itself...
                let mut found_matching_link = false;
                let mut title_attr = None;

                for attr in tag.html_attributes() {
                    let attr = attr?;
                    match attr.key.as_ref() {
                        b"href" => {
                            let href = attr_value(attr, &reader)?;
                            if href == item_href {
                                found_matching_link = true;
                            }
                            // Handle relative URLs.
                            if let Some(item_href_path) = url_path(item_href) {
                                if href == item_href_path {
                                    found_matching_link = true;
                                }
                            }
                        }
                        b"title" => {
                            title_attr = Some(attr);
                        }
                        _ => {}
                    }
                }
                if found_matching_link {
                    // ...then return the title of that link, if it has one.
                    if let Some(title_attr) = title_attr {
                        let title = attr_value(title_attr, &reader)?;
                        return Ok(Some(title.into_owned()));
                    }
                }
            }
            Event::Text(_) => {}
            _ => {}
        }
    }

    Ok(None)
}

fn attr_value<'a>(attr: Attribute<'a>, reader: &Reader<&[u8]>) -> Result<Cow<'a, str>, Error> {
    // Try to properly decode the value
    if let Ok(value) = attr.decode_and_unescape_value(reader) {
        return Ok(value);
    }

    // Some feeds use unescaped entities in their URLs; just use the raw content in that case.
    match attr.value {
        Cow::Borrowed(value) => Ok(Cow::Borrowed(str::from_utf8(value)?)),
        Cow::Owned(value) => Ok(Cow::Owned(String::from_utf8(value)?)),
    }
}

fn url_path(url: &str) -> Option<&str> {
    let url = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))?;
    Some(&url[url.find('/')?..])
}
