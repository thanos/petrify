use crate::types::ResourceType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub url: String,
    pub depth: String,
    pub max_pages: usize,
    pub download_only: Vec<String>,
    pub max_concurrent: usize,
    pub output: String,
    pub download_external: bool,
    pub convert_to_webp: bool,
    pub webp_quality: u8,
    pub webp_lossless: bool,
    pub ignore_robots: bool,
    pub timeout: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

impl Config {
    pub fn new() -> Self {
        Self {
            url: String::new(),
            depth: "unlimited".to_string(),
            max_pages: 0,
            download_only: vec![
                "js".to_string(),
                "css".to_string(),
                "images".to_string(),
                "video".to_string(),
                "html".to_string(),
                "pdf".to_string(),
                "fonts".to_string(),
            ],
            max_concurrent: num_cpus::get(),
            output: "./petrified_site".to_string(),
            download_external: true,
            convert_to_webp: true,
            webp_quality: 75,
            webp_lossless: false,
            ignore_robots: true,
            timeout: 270,
        }
    }

    pub fn should_download(&self, resource_type: &str) -> bool {
        self.download_only.iter().any(|t| t == resource_type)
    }

    pub fn should_download_type(&self, resource_type: &ResourceType) -> bool {
        self.should_download(resource_type.download_filter_key())
    }

    pub fn get_depth_limit(&self) -> Option<usize> {
        if self.depth == "unlimited" {
            None
        } else {
            self.depth.parse().ok()
        }
    }

    pub fn should_limit_pages(&self) -> bool {
        self.max_pages > 0
    }

    pub fn is_page_limit_reached(&self, current_count: usize) -> bool {
        self.should_limit_pages() && current_count >= self.max_pages
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_matches_new() {
        assert_eq!(Config::default().output, Config::new().output);
        assert_eq!(Config::default().timeout, Config::new().timeout);
    }
}
