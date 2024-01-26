use super::*;

#[test]
fn summary_from_html_summary_return_first_text() {
    let summary = summary_from_html_summary(
        r#"
        <div>
            <p>First paragraph.</p>
            <p>Second paragraph.</p>
        </div>
    "#,
    );

    assert_eq!(summary.unwrap(), Some(String::from("First paragraph.")));
}

#[test]
fn summary_from_html_summary_none_matching() {
    let summary = summary_from_html_summary(
        r#"
        <div>
            <img title="foo">
        </div>
    "#,
    );

    assert_eq!(summary.unwrap(), None);
}

#[test]
fn summary_from_html_summary_unescape() {
    let summary = summary_from_html_summary(
        r#"
        <div>
            &lt;foo&gt;
        </div>
    "#,
    );

    assert_eq!(summary.unwrap(), Some(String::from("<foo>")));
}

#[test]
fn summary_from_html_body_matching_link() {
    let summary = summary_from_html_body(
        "https://example.com",
        r#"
        <div>
            <a title="Test title" href="https://example.com">First paragraph.</a>
        </div>
    "#,
    );

    assert_eq!(summary.unwrap(), Some(String::from("Test title")));
}

#[test]
fn summary_from_html_body_matching_link_full_url() {
    let summary = summary_from_html_body(
        "https://example.com/foobar",
        r#"
        <div>
            <a title="Test title" href="https://example.com/foobar">First paragraph.</a>
        </div>
    "#,
    );

    assert_eq!(summary.unwrap(), Some(String::from("Test title")));
}

#[test]
fn summary_from_html_body_matching_link_path_only() {
    let summary = summary_from_html_body(
        "https://example.com/foobar",
        r#"
        <div>
            <a title="Test title" href="/foobar">First paragraph.</a>
        </div>
    "#,
    );

    assert_eq!(summary.unwrap(), Some(String::from("Test title")));
}

#[test]
fn summary_from_html_body_wrong_url() {
    let summary = summary_from_html_body(
        "https://example.com",
        r#"
        <div>
            <a title="Test title" href="https://example.net">First paragraph.</a>
        </div>
    "#,
    );

    assert_eq!(summary.unwrap(), None);
}

#[test]
fn summary_from_html_body_improperly_escaped_url() {
    let summary = summary_from_html_body(
        "https://example.com?foo&bar",
        r#"
        <div>
            <a title="Test title 2" href="https://example.com?foo&bar">First paragraph.</a>
        </div>
    "#,
    );

    assert_eq!(summary.unwrap(), Some(String::from("Test title 2")));
}
