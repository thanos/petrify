use petrify::config::Config;
use petrify::types::ResourceType;

#[test]
fn default_download_only_includes_fonts_and_media() {
    let config = Config::new();
    for expected in [
        "js", "css", "images", "video", "html", "pdf", "fonts",
    ] {
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

    config.depth = "3".to_string();
    assert_eq!(config.get_depth_limit(), Some(3));
}
