use petrify::config::Config;
use petrify::engine::Petrifier;
use std::fs;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const INDEX_HTML: &str = r#"<!DOCTYPE html>
<html>
<head><link rel="stylesheet" href="style.css"></head>
<body>
<img src="logo.png" alt="logo">
<script src="script.js"></script>
<a href="about.html">About</a>
</body>
</html>"#;

const ABOUT_HTML: &str = r#"<!DOCTYPE html><html><body><p>About</p><link rel="stylesheet" href="style.css"></body></html>"#;

fn tiny_png() -> Vec<u8> {
    let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([0, 128, 255, 255]));
    let mut buf = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut buf),
        image::ImageOutputFormat::Png,
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
        .respond_with(ResponseTemplate::new(200).set_body_string("body { color: black; }"))
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
    assert!(index_body.contains("static/css/style.css"));
    assert!(index_body.contains("static/js/script.js"));

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
}
