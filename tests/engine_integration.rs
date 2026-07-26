use petrify::config::Config;
use petrify::engine::Petrifier;
use std::fs;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const INDEX_HTML: &str = r#"<!DOCTYPE html>
<html>
<head>
<link rel="stylesheet" href="style.css">
<meta property="og:image" content="/og.png">
<meta name="twitter:image" content="/og.png">
</head>
<body>
<img src="logo.png" alt="logo">
<div style="background-image: url(/hero.jpg);"></div>
<img class="uk-invisible" src="" alt="">
<a href="/#section">Section</a>
<a href="about.html">About</a>
<script src="script.js"></script>
<link rel="preload" href="/fonts/brand.woff2" as="font" type="font/woff2">
</body>
</html>"#;

const ABOUT_HTML: &str = r#"<!DOCTYPE html><html><body>
<p>About</p>
<a href="https://cdn.example.com/remote.png">remote</a>
<img src="https://cdn.example.com/remote.png" alt="remote">
<link rel="stylesheet" href="style.css">
</body></html>"#;

fn tiny_png() -> Vec<u8> {
    let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([0, 128, 255, 255]));
    let mut buf = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
        .unwrap();
    buf
}

fn tiny_jpeg() -> Vec<u8> {
    let img = image::RgbImage::from_pixel(1, 1, image::Rgb([255, 0, 0]));
    let mut buf = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut buf),
        image::ImageFormat::Jpeg,
    )
    .unwrap();
    buf
}

async fn mount_test_site(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(INDEX_HTML))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/style.css"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            "@font-face { font-family: Brand; src: url(/fonts/brand.woff2); } body { color: black; }",
        ))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/script.js"))
        .respond_with(ResponseTemplate::new(200).set_body_string("console.log('petrify');"))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/logo.png"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(tiny_png()))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/og.png"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(tiny_png()))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/hero.jpg"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(tiny_jpeg()))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/fonts/brand.woff2"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(b"wOFF2fake")
                .insert_header("content-type", "font/woff2"),
        )
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/about.html"))
        .respond_with(ResponseTemplate::new(200).set_body_string(ABOUT_HTML))
        .mount(server)
        .await;
}

fn test_config(server_uri: &str, output: &str) -> Config {
    let mut config = Config::new();
    config.url = server_uri.to_string();
    config.output = output.to_string();
    config.max_concurrent = 2;
    config.convert_to_webp = false;
    config.timeout = 10;
    config
}

#[tokio::test]
async fn run_petrifies_site_end_to_end() {
    let server = MockServer::start().await;
    mount_test_site(&server).await;

    let dir = TempDir::new().unwrap();
    let output = dir.path().join("out").to_string_lossy().to_string();
    let mut config = test_config(&server.uri(), &output);
    config.max_pages = 0;

    let mut petrifier = Petrifier::new(config).await.unwrap();
    petrifier.run().await.unwrap();

    let index = dir.path().join("out/index.html");
    assert!(index.exists(), "index.html should be written");
    let index_body = fs::read_to_string(index).unwrap();
    assert!(index_body.contains("/static/css/style.css"));
    assert!(index_body.contains("/static/js/script.js"));
    assert!(index_body.contains("</body>"));
    assert!(!index_body.contains("//static/"));

    assert!(dir.path().join("out/static/css/style.css").exists());
    assert!(dir.path().join("out/static/js/script.js").exists());
    assert!(dir.path().join("out/static/images/logo.png").exists());
    assert!(dir.path().join("out/about.html").exists());
}

#[tokio::test]
async fn run_respects_max_pages_limit() {
    let server = MockServer::start().await;
    mount_test_site(&server).await;

    let dir = TempDir::new().unwrap();
    let output = dir.path().join("out").to_string_lossy().to_string();
    let mut config = test_config(&server.uri(), &output);
    config.max_pages = 1;

    let mut petrifier = Petrifier::new(config).await.unwrap();
    petrifier.run().await.unwrap();

    assert!(dir.path().join("out/index.html").exists());
    // about.html should not be processed when page limit is 1
    assert!(!dir.path().join("out/about.html").exists());
}

#[tokio::test]
async fn run_honors_download_only_filter() {
    let server = MockServer::start().await;
    mount_test_site(&server).await;

    let dir = TempDir::new().unwrap();
    let output = dir.path().join("out").to_string_lossy().to_string();
    let mut config = test_config(&server.uri(), &output);
    config.max_pages = 1;
    config.download_only = vec!["html".to_string()];

    let mut petrifier = Petrifier::new(config).await.unwrap();
    petrifier.run().await.unwrap();

    assert!(dir.path().join("out/index.html").exists());
    assert!(!dir.path().join("out/static/css/style.css").exists());
    assert!(!dir.path().join("out/static/js/script.js").exists());
}

#[tokio::test]
async fn run_converts_images_to_webp_when_enabled() {
    let server = MockServer::start().await;
    mount_test_site(&server).await;

    let dir = TempDir::new().unwrap();
    let output = dir.path().join("out").to_string_lossy().to_string();
    let mut config = test_config(&server.uri(), &output);
    config.max_pages = 1;
    config.convert_to_webp = true;
    config.download_only = vec!["html".to_string(), "images".to_string()];

    let mut petrifier = Petrifier::new(config).await.unwrap();
    petrifier.run().await.unwrap();

    assert!(dir.path().join("out/static/images/logo.webp").exists());
    assert!(!dir.path().join("out/static/images/logo.png").exists());

    let index_body = fs::read_to_string(dir.path().join("out/index.html")).unwrap();
    assert!(
        index_body.contains("/static/images/logo.webp"),
        "HTML should reference the converted webp path"
    );
    assert!(!index_body.contains("logo.png"));
}

#[tokio::test]
async fn run_downloads_seo_meta_and_style_background_images() {
    let server = MockServer::start().await;
    mount_test_site(&server).await;

    let dir = TempDir::new().unwrap();
    // Trailing slash previously produced //static/... protocol-relative URLs.
    let output = dir.path().join("out").to_string_lossy().to_string() + "/";
    let mut config = test_config(&server.uri(), &output);
    config.max_pages = 1;
    config.convert_to_webp = true;

    let mut petrifier = Petrifier::new(config).await.unwrap();
    petrifier.run().await.unwrap();

    let index_body = fs::read_to_string(dir.path().join("out/index.html")).unwrap();
    assert!(index_body.contains(r#"content="/static/images/og.webp""#));
    assert!(index_body.contains("url(/static/images/hero.webp)"));
    assert!(index_body.contains(r#"href="/index.html#section""#));
    assert!(!index_body.contains("%23"));
    assert!(!index_body.contains("//static/"));
    assert!(index_body.contains("</body>"));
    assert!(
        index_body.contains(r#"src="""#),
        "empty src should be preserved"
    );

    assert!(dir.path().join("out/static/images/og.webp").exists());
    assert!(dir.path().join("out/static/images/hero.webp").exists());
}

#[tokio::test]
async fn run_downloads_fonts_by_default() {
    let server = MockServer::start().await;
    mount_test_site(&server).await;

    let dir = TempDir::new().unwrap();
    let output = dir.path().join("out").to_string_lossy().to_string();
    let mut config = test_config(&server.uri(), &output);
    config.max_pages = 1;
    // Use Config::new() defaults for download_only (includes fonts).
    assert!(
        config.download_only.iter().any(|t| t == "fonts"),
        "fonts should be in the default download list"
    );

    let mut petrifier = Petrifier::new(config).await.unwrap();
    petrifier.run().await.unwrap();

    assert!(
        dir.path().join("out/static/fonts/brand.woff2").exists(),
        "font file should be downloaded with default filters"
    );
    let index_body = fs::read_to_string(dir.path().join("out/index.html")).unwrap();
    assert!(index_body.contains("/static/fonts/brand.woff2"));
}

#[tokio::test]
async fn run_respects_depth_zero_starting_page_only() {
    let server = MockServer::start().await;
    mount_test_site(&server).await;

    let dir = TempDir::new().unwrap();
    let output = dir.path().join("out").to_string_lossy().to_string();
    let mut config = test_config(&server.uri(), &output);
    config.depth = "0".to_string();

    let mut petrifier = Petrifier::new(config).await.unwrap();
    petrifier.run().await.unwrap();

    assert!(dir.path().join("out/index.html").exists());
    assert!(
        !dir.path().join("out/about.html").exists(),
        "depth 0 must not follow links to about.html"
    );
}

#[tokio::test]
async fn run_deep_unlimited_follows_linked_pages() {
    let server = MockServer::start().await;
    mount_test_site(&server).await;

    let dir = TempDir::new().unwrap();
    let output = dir.path().join("out").to_string_lossy().to_string();
    let mut config = test_config(&server.uri(), &output);
    config.depth = "unlimited".to_string();

    let mut petrifier = Petrifier::new(config).await.unwrap();
    petrifier.run().await.unwrap();

    assert!(dir.path().join("out/index.html").exists());
    assert!(dir.path().join("out/about.html").exists());
}

#[tokio::test]
async fn run_stay_on_site_skips_third_party_assets() {
    let server = MockServer::start().await;
    mount_test_site(&server).await;

    let dir = TempDir::new().unwrap();
    let output = dir.path().join("out").to_string_lossy().to_string();
    let mut config = test_config(&server.uri(), &output);
    config.download_external = false;
    config.convert_to_webp = true;
    config.download_only = vec!["html".to_string(), "images".to_string(), "css".to_string()];

    let mut petrifier = Petrifier::new(config).await.unwrap();
    petrifier.run().await.unwrap();

    assert!(dir.path().join("out/about.html").exists());
    // Same-host assets still download
    assert!(dir.path().join("out/static/images/logo.webp").exists());
    // Third-party CDN image from about.html must not be fetched
    let about_body = fs::read_to_string(dir.path().join("out/about.html")).unwrap();
    assert!(
        about_body.contains("cdn.example.com/remote.png")
            || !dir.path().join("out/static/images/remote.webp").exists(),
        "off-site CDN assets must not be downloaded when stay-on-site"
    );
    assert!(!dir.path().join("out/static/images/remote.webp").exists());
    assert!(!dir.path().join("out/static/images/remote.png").exists());
}

#[tokio::test]
async fn run_mirrors_absolute_same_host_images_as_site_root_webp() {
    let server = MockServer::start().await;

    let about_with_abs = format!(
        r#"<!DOCTYPE html><html><body>
<img src="{}/cdn/remote.png" alt="remote">
</body></html>"#,
        server.uri()
    );

    Mock::given(method("GET"))
        .and(path("/about.html"))
        .respond_with(ResponseTemplate::new(200).set_body_string(about_with_abs))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/cdn/remote.png"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(tiny_png()))
        .mount(&server)
        .await;

    let dir = TempDir::new().unwrap();
    let output = dir.path().join("out").to_string_lossy().to_string();
    let mut config = test_config(&server.uri(), &output);
    config.url = format!("{}/about.html", server.uri());
    config.max_pages = 1;
    config.convert_to_webp = true;
    config.download_only = vec!["html".to_string(), "images".to_string()];

    let mut petrifier = Petrifier::new(config).await.unwrap();
    petrifier.run().await.unwrap();

    assert!(dir.path().join("out/about.html").exists());
    assert!(dir.path().join("out/static/images/remote.webp").exists());
    let about_body = fs::read_to_string(dir.path().join("out/about.html")).unwrap();
    assert!(about_body.contains(r#"src="/static/images/remote.webp""#));
    assert!(!about_body.contains("/cdn/remote.png"));
}
