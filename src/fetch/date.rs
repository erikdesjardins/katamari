use chrono::{DateTime, Utc};

// Fri, 01 August 2025 07:00:00 GMT
// before parsing, converted to:
// 01 August 2025 07:00:00 +0000
const RFC1123: &str = "%d %B %Y %H:%M:%S %z";

// 2025-08-26 21:29:20 UTC
// before parsing, converted to:
// 2025-08-26 21:29:20 +0000
const GITHUB_DATE: &str = "%Y-%m-%d %H:%M:%S %z";

/// Similar to feed_rs::util::parse_timestamp_lenient, but with support for GitHub's nonstandard timestamp format
pub fn parse_date(raw_input: &str) -> Option<DateTime<Utc>> {
    let input = raw_input
        .replace("GMT", "+0000")
        .replace("UTC", "+0000")
        .replace("Mon, ", "")
        .replace("Tue, ", "")
        .replace("Wed, ", "")
        .replace("Thu, ", "")
        .replace("Fri, ", "")
        .replace("Sat, ", "")
        .replace("Sun, ", "");

    let raw_timestamp = DateTime::parse_from_rfc3339(&input)
        .or_else(|_| DateTime::parse_from_rfc2822(&input))
        .or_else(|_| DateTime::parse_from_str(&input, RFC1123))
        .or_else(|_| DateTime::parse_from_str(&input, GITHUB_DATE));

    match raw_timestamp {
        Ok(timestamp) => Some(timestamp.with_timezone(&Utc)),
        Err(_) => {
            tracing::debug!("Failed to parse date: '{raw_input}' -> '{input}'");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_date() {
        let cases = [
            // RFC3339
            ("2023-01-07T17:12:42+00:00", "2023-01-07T17:12:42Z"),
            ("2025-05-01T10:17:04.634+03:00", "2025-05-01T07:17:04.634Z"),
            (
                "2025-08-24T12:33:09.842034+00:00",
                "2025-08-24T12:33:09.842034Z",
            ),
            ("2024-11-19T08:58:00Z", "2024-11-19T08:58:00Z"),
            ("2025-07-30T13:00:00.000Z", "2025-07-30T13:00:00Z"),
            // RFC2822
            ("Wed, 02 May 2025 07:00:00 GMT", "2025-05-02T07:00:00Z"),
            ("Thu, 07 Dec 2023 00:00:00 +0000", "2023-12-07T00:00:00Z"),
            ("Tue, 08 Apr 2025 11:03:32 +0200", "2025-04-08T09:03:32Z"),
            // RDS1123-like
            ("Fri, 01 August 2025 07:00:00 GMT", "2025-08-01T07:00:00Z"),
            // GitHub
            ("2025-08-26 21:29:20 UTC", "2025-08-26T21:29:20Z"),
            ("2025-08-26 05:37:56 -0700", "2025-08-26T12:37:56Z"),
        ];

        for (input, expected) in cases {
            let parsed = parse_date(input);
            let expected: DateTime<Utc> = expected.parse().unwrap();
            assert_eq!(parsed, Some(expected), "input: '{input}'");
        }
    }
}
