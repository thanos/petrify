use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ResourceType {
    HTML,
    CSS,
    JavaScript,
    Image,
    Video,
    PDF,
    Font,
    Other,
}

impl std::fmt::Display for ResourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.download_filter_key())
    }
}

impl ResourceType {
    /// CLI filter name used by `--download-only` (e.g. `js`, not `javascript`).
    pub fn download_filter_key(&self) -> &'static str {
        match self {
            ResourceType::HTML => "html",
            ResourceType::CSS => "css",
            ResourceType::JavaScript => "js",
            ResourceType::Image => "images",
            ResourceType::Video => "video",
            ResourceType::PDF => "pdf",
            ResourceType::Font => "fonts",
            ResourceType::Other => "other",
        }
    }
}

impl From<&str> for ResourceType {
    fn from(mime_type: &str) -> Self {
        match mime_type {
            "text/html" => ResourceType::HTML,
            "text/css" => ResourceType::CSS,
            "application/javascript" | "text/javascript" => ResourceType::JavaScript,
            mime if mime.starts_with("image/") => ResourceType::Image,
            mime if mime.starts_with("video/") => ResourceType::Video,
            "application/pdf" => ResourceType::PDF,
            mime if mime.starts_with("font/") || mime == "application/font-woff" => ResourceType::Font,
            _ => ResourceType::Other,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub url: Url,
    pub local_path: String,
    pub resource_type: ResourceType,
    pub mime_type: String,
    pub size: Option<u64>,
    pub downloaded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub url: Url,
    pub local_path: String,
    pub depth: usize,
    pub resources: Vec<Resource>,
    pub processed: bool,
}

#[derive(Debug, Clone)]
pub struct DownloadStats {
    pub total_pages: usize,
    pub processed_pages: usize,
    pub total_resources: usize,
    pub downloaded_resources: usize,
    pub total_size: u64,
    pub start_time: std::time::Instant,
}

impl Default for DownloadStats {
    fn default() -> Self {
        Self::new()
    }
}

impl DownloadStats {
    pub fn new() -> Self {
        Self {
            total_pages: 0,
            processed_pages: 0,
            total_resources: 0,
            downloaded_resources: 0,
            total_size: 0,
            start_time: std::time::Instant::now(),
        }
    }

    pub fn progress_percentage(&self) -> f64 {
        if self.total_pages == 0 {
            0.0
        } else {
            (self.processed_pages as f64 / self.total_pages as f64) * 100.0
        }
    }

    pub fn elapsed_time(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }
}

#[derive(Debug, Clone)]
pub struct WorkQueue {
    pub pages: Vec<Url>,
    pub resources: Vec<Url>,
    pub visited_urls: HashMap<String, bool>,
}

impl Default for WorkQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkQueue {
    pub fn new() -> Self {
        Self {
            pages: Vec::new(),
            resources: Vec::new(),
            visited_urls: HashMap::new(),
        }
    }

    pub fn add_page(&mut self, url: Url) {
        let normalized = self.normalize_url(&url);
        if !self.visited_urls.contains_key(&normalized) {
            self.pages.push(url);
            self.visited_urls.insert(normalized, true);
        }
    }

    pub fn add_resource(&mut self, url: Url) {
        // Resources can be added multiple times (they might be referenced from different pages)
        // Only check if we already have this exact resource to avoid duplicates
        let normalized = self.normalize_url(&url);
        if !self.resources.iter().any(|r| self.normalize_url(r) == normalized) {
            self.resources.push(url);
        }
    }

    pub fn normalize_url(&self, url: &Url) -> String {
        let mut normalized = url.clone();
        normalized.set_fragment(None);
        normalized.set_query(None);
        normalized.to_string()
    }

    pub fn get_next_page(&mut self) -> Option<Url> {
        self.pages.pop()
    }

    pub fn get_next_resource(&mut self) -> Option<Url> {
        self.resources.pop()
    }

    pub fn is_empty(&self) -> bool {
        self.pages.is_empty() && self.resources.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn download_filter_key_matches_cli_values() {
        assert_eq!(ResourceType::JavaScript.download_filter_key(), "js");
        assert_eq!(ResourceType::Image.download_filter_key(), "images");
        assert_eq!(ResourceType::Font.download_filter_key(), "fonts");
    }

    #[test]
    fn display_uses_download_filter_key() {
        assert_eq!(ResourceType::JavaScript.to_string(), "js");
    }

    #[test]
    fn download_stats_progress_percentage() {
        let mut stats = DownloadStats::new();
        assert_eq!(stats.progress_percentage(), 0.0);

        stats.total_pages = 4;
        stats.processed_pages = 2;
        assert_eq!(stats.progress_percentage(), 50.0);
    }

    #[test]
    fn resource_type_from_mime() {
        assert!(matches!(
            ResourceType::from("text/css"),
            ResourceType::CSS
        ));
        assert!(matches!(
            ResourceType::from("image/png"),
            ResourceType::Image
        ));
        assert!(matches!(
            ResourceType::from("application/octet-stream"),
            ResourceType::Other
        ));
    }

    #[test]
    fn work_queue_fifo_and_empty() {
        let mut queue = WorkQueue::new();
        assert!(queue.is_empty());

        queue.add_page(Url::parse("https://example.com/a").unwrap());
        queue.add_resource(Url::parse("https://example.com/b.js").unwrap());
        assert!(!queue.is_empty());

        assert_eq!(
            queue.get_next_page().unwrap().path(),
            "/a"
        );
        assert_eq!(
            queue.get_next_resource().unwrap().path(),
            "/b.js"
        );
        assert!(queue.is_empty());
    }
} 