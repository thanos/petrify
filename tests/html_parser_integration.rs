use petrify::html_parser::HtmlParser;
use petrify::types::ResourceType;
use url::Url;

const FIXTURE_HTML: &str = include_str!("../test_site/index.html");

#[test]
fn extracts_css_js_and_images_from_fixture() {
    let base_url = Url::parse("https://example.com/").unwrap();
    let parser = HtmlParser::new(base_url, "./output".to_string(), false);

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
            .any(|r| r.resource_type == ResourceType::JavaScript
                && r.url.path().ends_with("script.js")),
        "expected javascript resource"
    );
    assert!(
        resources
            .iter()
            .filter(|r| r.resource_type == ResourceType::Image)
            .count()
            >= 2,
        "expected at least two image resources"
    );
}

#[test]
fn rewrites_urls_to_site_root_paths() {
    let base_url = Url::parse("https://example.com/").unwrap();
    let parser = HtmlParser::new(base_url, "./mirrored".to_string(), false);

    let (modified, _) = parser.parse_html(FIXTURE_HTML).unwrap();

    assert!(modified.contains("/static/css/style.css"));
    assert!(modified.contains("/static/js/script.js"));
    assert!(!modified.contains("../static/"));
    assert!(!modified.contains("https://example.com/style.css"));
}

#[test]
fn discovers_extensionless_html_links() {
    let html =
        r#"<html><body><a href="/about">About</a><a href="/contact">Contact</a></body></html>"#;
    let base_url = Url::parse("https://example.com/").unwrap();
    let parser = HtmlParser::new(base_url, "./output".to_string(), false);

    let (_, resources) = parser.parse_html(html).unwrap();
    assert!(resources
        .iter()
        .any(|r| r.resource_type == ResourceType::HTML && r.url.path() == "/about"));
}

#[test]
fn empty_src_and_root_href_do_not_corrupt_html() {
    let html = r#"<!DOCTYPE html><html><head></head><body>
        <a href="/">Home</a>
        <img class="uk-invisible" src="" width="" height="" alt="">
        <p>Visit /docs in plain text</p>
        </body></html>"#;
    let base_url = Url::parse("http://127.0.0.1:8000/nested/page/").unwrap();
    let parser = HtmlParser::new(base_url, "./mb/".to_string(), false);
    let (modified, _) = parser.parse_html(html).unwrap();

    assert!(modified.contains("<!DOCTYPE html>"));
    assert!(modified.contains("</body>"));
    assert!(modified.contains("</html>"));
    assert!(modified.contains(r#"href="/index.html""#));
    assert!(modified.contains(r#"src="""#));
    assert!(modified.contains("/docs"));
    assert!(!modified.contains("%23"));
    assert!(!modified.contains("//static/"));
    assert!(!modified.contains("static/other/"));
}

#[test]
fn style_attribute_background_images_are_discovered_and_rewritten() {
    let html = r#"<html><body>
        <div style="background-image: url(/static/images/2025/hero.webp);"></div>
        <style>.home { background-image: url(/static/images/home.jpg); }</style>
    </body></html>"#;
    let base_url = Url::parse("http://127.0.0.1:8000/2025-9-the-amphibian/").unwrap();
    let parser = HtmlParser::new(base_url, "./mb".to_string(), false);
    let (modified, resources) = parser.parse_html(html).unwrap();

    assert!(resources
        .iter()
        .any(|r| r.url.path().ends_with("hero.webp")));
    assert!(resources.iter().any(|r| r.url.path().ends_with("home.jpg")));
    assert!(modified.contains("url(/static/images/hero.webp)"));
    assert!(modified.contains("url(/static/images/home.jpg)"));
    assert!(!modified.contains("/static/images/2025/hero.webp"));
}

#[test]
fn fragment_urls_are_rewritten_without_percent23_html_files() {
    let html = r##"<html><body>
        <a href="/2025-9-the-amphibian/#home">Home</a>
        <a href="/2025-9-the-amphibian/#program">Program</a>
        <a href="#local">Local</a>
    </body></html>"##;
    let base_url = Url::parse("http://127.0.0.1:8000/").unwrap();
    let parser = HtmlParser::new(base_url, "./mb".to_string(), false);
    let (modified, resources) = parser.parse_html(html).unwrap();

    let html_pages: Vec<_> = resources
        .iter()
        .filter(|r| r.resource_type == ResourceType::HTML)
        .collect();
    assert_eq!(html_pages.len(), 1);
    assert!(html_pages[0]
        .local_path
        .ends_with("2025-9-the-amphibian/index.html"));
    assert!(!html_pages[0].local_path.contains("%23"));

    assert!(modified.contains(r#"href="/2025-9-the-amphibian/index.html#home""#));
    assert!(modified.contains(r#"href="/2025-9-the-amphibian/index.html#program""#));
    assert!(modified.contains(r##"href="#local""##));
    assert!(!modified.contains("%23home"));
    assert!(!modified.contains("#home.html"));
}

#[test]
fn seo_meta_and_json_ld_images_are_discovered_and_rewritten() {
    let html = r#"<html><head>
        <meta property="og:image" content="https://s3.amazonaws.com/bucket/og-cover.jpeg" />
        <meta name="twitter:image" content="https://s3.amazonaws.com/bucket/og-cover.jpeg">
        <meta name="description" content="Plain text, not a URL">
        <script type="application/ld+json">
        {"@type":"Person","name":"Artist","image":"https://cdn.example.com/portrait.png","url":"https://example.com/artist"}
        </script>
    </head><body></body></html>"#;
    let base_url = Url::parse("http://127.0.0.1:8000/artfestival/artist/x/").unwrap();
    let parser = HtmlParser::new(base_url, "./mb".to_string(), true);
    let (modified, resources) = parser.parse_html(html).unwrap();

    assert!(resources.iter().any(|r| {
        r.resource_type == ResourceType::Image && r.url.path().ends_with("og-cover.jpeg")
    }));
    assert!(resources.iter().any(|r| {
        r.resource_type == ResourceType::Image && r.url.path().ends_with("portrait.png")
    }));
    assert!(!resources.iter().any(|r| r.url.path() == "/artist"));

    assert!(modified.contains(r#"content="/static/images/og-cover.webp""#));
    assert!(modified.contains(r#""image":"/static/images/portrait.webp""#));
    assert!(modified.contains(r#"content="Plain text, not a URL""#));
    assert!(modified.contains(r#""url":"https://example.com/artist""#));
}

#[test]
fn external_images_rewrite_to_webp_site_root_paths() {
    let html = r#"<html><body>
        <a href="https://s3.amazonaws.com/bucket/full.JPG"><img src="https://s3.amazonaws.com/bucket/thumb.jpg"></a>
    </body></html>"#;
    let base_url = Url::parse("http://127.0.0.1:8000/artfestival/artist/raed-issa/").unwrap();
    let parser = HtmlParser::new(base_url, "./mb".to_string(), true);
    let (modified, resources) = parser.parse_html(html).unwrap();

    assert_eq!(
        resources
            .iter()
            .filter(|r| r.resource_type == ResourceType::Image)
            .count(),
        2
    );
    assert!(resources.iter().all(|r| r.local_path.ends_with(".webp")));
    assert!(modified.contains(r#"src="/static/images/thumb.webp""#));
    assert!(modified.contains(r#"href="/static/images/full.webp""#));
    assert!(!modified.contains("../static/"));
    assert!(!modified.contains("thumb.jpg"));
    assert!(!modified.contains("full.JPG"));
}

#[test]
fn fonts_are_classified_as_font_resources() {
    let html = r#"<html><head>
        <link rel="stylesheet" href="/static/fonts/site.woff2">
        <style>@font-face { src: url(/assets/brand.ttf); }</style>
    </head></html>"#;
    let base_url = Url::parse("https://example.com/").unwrap();
    let parser = HtmlParser::new(base_url, "./output".to_string(), false);
    let (modified, resources) = parser.parse_html(html).unwrap();

    assert!(resources.iter().any(|r| {
        r.resource_type == ResourceType::Font && r.url.path().ends_with("site.woff2")
    }));
    assert!(resources
        .iter()
        .any(|r| { r.resource_type == ResourceType::Font && r.url.path().ends_with("brand.ttf") }));
    assert!(modified.contains("/static/fonts/site.woff2"));
    assert!(modified.contains("url(/static/fonts/brand.ttf)"));
}

#[test]
fn offsite_html_links_are_left_absolute_while_assets_are_mirrored() {
    let html = r#"<html><body>
        <a href="https://other.example/about">About</a>
        <a href="https://other.example/docs/guide.html">Guide</a>
        <a href="https://cdn.example/lib.js">script link</a>
        <img src="https://cdn.example/photo.png" alt="">
        <link rel="stylesheet" href="https://cdn.example/theme.css">
        <script src="https://cdn.example/app.js"></script>
        <a href="https://cdn.example/report.pdf">PDF</a>
        <a href="https://cdn.example/notes.docx">Doc</a>
    </body></html>"#;
    let base_url = Url::parse("http://127.0.0.1:8001/bio/").unwrap();
    let parser = HtmlParser::new(base_url, "./venieri".to_string(), true);
    let (modified, resources) = parser.parse_html(html).unwrap();

    assert!(
        !resources
            .iter()
            .any(|r| r.resource_type == ResourceType::HTML),
        "off-site HTML pages must not be queued"
    );
    assert!(resources
        .iter()
        .any(|r| r.resource_type == ResourceType::Image));
    assert!(resources.iter().any(|r| r.resource_type == ResourceType::CSS));
    assert!(resources
        .iter()
        .any(|r| r.resource_type == ResourceType::JavaScript));
    assert!(resources.iter().any(|r| r.resource_type == ResourceType::PDF));

    assert!(modified.contains(r#"href="https://other.example/about""#));
    assert!(modified.contains(r#"href="https://other.example/docs/guide.html""#));
    assert!(modified.contains(r#"src="/static/images/photo.webp""#));
    assert!(modified.contains(r#"href="/static/css/theme.css""#));
    assert!(modified.contains(r#"src="/static/js/app.js""#));
    assert!(modified.contains(r#"href="/static/pdf/report.pdf""#));
    assert!(modified.contains(r#"href="/static/pdf/notes.docx""#));
}
