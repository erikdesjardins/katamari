pub fn scheme(url: &str) -> &str {
    url.split_once("://")
        .map(|(scheme, _)| scheme)
        .unwrap_or("https")
}

pub fn domain(url: &str) -> &str {
    url.split_once("://")
        .map(|(_, rest)| {
            rest.split_once('/')
                .map(|(domain, _)| domain)
                .unwrap_or(rest)
        })
        .unwrap_or(url)
}

pub fn prefix(url: &str) -> &str {
    url.split_once('#').map(|(prefix, _)| prefix).unwrap_or(url)
}

pub fn make_absolute(base: &str, url: String) -> String {
    if url.contains("://") {
        return url;
    }

    let scheme = scheme(base);
    let domain = domain(base);
    let path = url.trim_start_matches('/');
    format!("{scheme}://{domain}/{path}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheme() {
        assert_eq!(scheme("https://example.com"), "https");
        assert_eq!(scheme("http://example.com"), "http");
        assert_eq!(scheme("example.com"), "https");
    }

    #[test]
    fn test_domain() {
        assert_eq!(domain("example.com"), "example.com");
        assert_eq!(domain("https://example.com"), "example.com");
        assert_eq!(domain("http://example.com/foo"), "example.com");
        assert_eq!(domain("https://example.com/foo/bar"), "example.com");
    }

    #[test]
    fn test_prefix() {
        assert_eq!(prefix("https://example.com"), "https://example.com");
        assert_eq!(
            prefix("https://example.com/test"),
            "https://example.com/test"
        );
        assert_eq!(prefix("http://example.com#foo"), "http://example.com");
        assert_eq!(
            prefix("http://example.com/test#foo"),
            "http://example.com/test"
        );
        assert_eq!(prefix("https://example.com#foo#bar"), "https://example.com");
        assert_eq!(
            prefix("https://example.com/test/2#foo#bar"),
            "https://example.com/test/2"
        );
    }

    #[test]
    fn test_make_absolute() {
        assert_eq!(
            make_absolute("https://example.com/feed.xml", "/foo".to_string()),
            "https://example.com/foo"
        );
        assert_eq!(
            make_absolute("https://example.com/feed.xml", "foo".to_string()),
            "https://example.com/foo"
        );
        assert_eq!(
            make_absolute("https://example.com", "/foo".to_string()),
            "https://example.com/foo"
        );
        assert_eq!(
            make_absolute("https://example.com", "foo".to_string()),
            "https://example.com/foo"
        );
        assert_eq!(
            make_absolute("https://example.com", "https://other.net/foo".to_string()),
            "https://other.net/foo"
        );
    }
}
