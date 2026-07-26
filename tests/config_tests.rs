use petrify::config::Config;
use petrify::types::ResourceType;

#[test]
fn default_download_only_includes_fonts_and_media() {
    let config = Config::new();
    for expected in ["js", "css", "images", "video", "html", "pdf", "fonts"] {
        assert!(
            config.download_only.iter().any(|t| t == expected),
            "default download_only missing {expected}"
        );
    }
}

#[test]
fn should_download_matches_cli_filter_names() {
    let config = Config::new();

    assert!(config.should_download_type(&ResourceType::JavaScript));
    assert!(config.should_download_type(&ResourceType::CSS));
    assert!(config.should_download_type(&ResourceType::Image));
    assert!(config.should_download_type(&ResourceType::Font));
}

#[test]
fn respects_custom_download_only_list() {
    let mut config = Config::new();
    config.download_only = vec!["html".to_string(), "js".to_string()];

    assert!(config.should_download_type(&ResourceType::HTML));
    assert!(config.should_download_type(&ResourceType::JavaScript));
    assert!(!config.should_download_type(&ResourceType::CSS));
}

#[test]
fn max_pages_limit_helpers() {
    let mut config = Config::new();
    assert!(!config.should_limit_pages());

    config.max_pages = 5;
    assert!(config.should_limit_pages());
    assert!(!config.is_page_limit_reached(4));
    assert!(config.is_page_limit_reached(5));
}

#[test]
fn depth_limit_parsing() {
    let mut config = Config::new();
    assert_eq!(config.get_depth_limit(), None);
    assert!(config.allows_depth(99));

    config.depth = "3".to_string();
    assert_eq!(config.get_depth_limit(), Some(3));
    assert!(config.allows_depth(0));
    assert!(config.allows_depth(3));
    assert!(!config.allows_depth(4));
}

#[test]
fn stay_on_site_rejects_off_host_urls() {
    let mut config = Config::new();
    let base = url::Url::parse("https://example.com/").unwrap();
    let same = url::Url::parse("https://example.com/static/a.png").unwrap();
    let other = url::Url::parse("https://cdn.example.com/a.png").unwrap();
    let other_page = url::Url::parse("https://cdn.example.com/about").unwrap();

    assert!(config.allows_url(&same, &base));
    assert!(config.allows_url(&other, &base));
    assert!(config.allows_resource(&other, &base, &ResourceType::Image));
    assert!(
        !config.allows_resource(&other_page, &base, &ResourceType::HTML),
        "off-site HTML pages must never be mirrored"
    );

    config.download_external = false;
    assert!(config.allows_url(&same, &base));
    assert!(!config.allows_url(&other, &base));
    assert!(!config.allows_resource(&other, &base, &ResourceType::Image));
}
