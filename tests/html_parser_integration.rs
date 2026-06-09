use petrify::html_parser::HtmlParser;
use petrify::types::ResourceType;
use url::Url;

const FIXTURE_HTML: &str = include_str!("../test_site/index.html");

#[test]
fn extracts_css_js_and_images_from_fixture() {
    let base_url = Url::parse("https://example.com/").unwrap();
    let parser = HtmlParser::new(base_url, "./output".to_string());

    let (_, resources) = parser.parse_html(FIXTURE_HTML).unwrap();

    assert!(
        resources
            .iter()
            .any(|r| r.resource_type == ResourceType::CSS && r.url.path().ends_with("style.css")),
        "expected stylesheet resource"
    );
    assert!(
        resources
            .iter()
            .any(|r| r.resource_type == ResourceType::JavaScript && r.url.path().ends_with("script.js")),
        "expected javascript resource"
    );
    assert!(
        resources
            .iter()
            .filter(|r| r.resource_type == ResourceType::Image)
            .count() >= 2,
        "expected at least two image resources"
    );
}

#[test]
fn rewrites_urls_to_local_paths() {
    let base_url = Url::parse("https://example.com/").unwrap();
    let parser = HtmlParser::new(base_url, "./mirrored".to_string());

    let (modified, _) = parser.parse_html(FIXTURE_HTML).unwrap();

    assert!(modified.contains("static/css/style.css"));
    assert!(modified.contains("static/js/script.js"));
    assert!(!modified.contains("https://example.com/style.css"));
}

#[test]
fn discovers_extensionless_html_links() {
    let html = r#"<html><body><a href="/about">About</a><a href="/contact">Contact</a></body></html>"#;
    let base_url = Url::parse("https://example.com/").unwrap();
    let parser = HtmlParser::new(base_url, "./output".to_string());

    let (_, resources) = parser.parse_html(html).unwrap();
    assert!(
        resources
            .iter()
            .any(|r| r.resource_type == ResourceType::HTML && r.url.path() == "/about")
    );
}
