use crate::types::{Resource, ResourceType};
use anyhow::{anyhow, Result};
use html5ever::parse_document;
use html5ever::tendril::TendrilSink;
use markup5ever_rcdom::{Handle, NodeData, RcDom};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct UrlReference {
    original: String,
    resolved: Url,
}

pub struct HtmlParser {
    base_url: Url,
    output_dir: String,
}

impl HtmlParser {
    pub fn new(base_url: Url, output_dir: String) -> Self {
        Self {
            base_url,
            output_dir,
        }
    }

    pub fn parse_html(&self, html_content: &str) -> Result<(String, Vec<Resource>)> {
        let dom = parse_document(RcDom::default(), Default::default())
            .from_utf8()
            .one(html_content.as_bytes());

        let mut resources = Vec::new();
        let mut modified_html = html_content.to_string();

        let references = self.extract_url_references(&dom.document)?;
        let mut local_paths = HashMap::new();

        for reference in &references {
            if local_paths.contains_key(reference.resolved.as_str()) {
                continue;
            }
            match self.create_resource(&reference.resolved) {
                Ok(resource) => {
                    local_paths.insert(reference.resolved.to_string(), resource.local_path.clone());
                    resources.push(resource);
                }
                Err(e) => {
                    log::warn!(
                        "Failed to create resource for {}: {}",
                        reference.resolved.as_str(),
                        e
                    );
                }
            }
        }

        for reference in &references {
            if let Some(local_path) = local_paths.get(reference.resolved.as_str()) {
                modified_html =
                    self.replace_url_in_html(&modified_html, &reference.original, local_path);
            }
        }

        Ok((modified_html, resources))
    }

    fn extract_url_references(&self, handle: &Handle) -> Result<Vec<UrlReference>> {
        let mut references = Vec::new();
        let mut seen = HashSet::new();
        self.walk_dom(handle, &mut references, &mut seen)?;
        Ok(references)
    }

    fn walk_dom(
        &self,
        handle: &Handle,
        references: &mut Vec<UrlReference>,
        seen: &mut HashSet<(String, String)>,
    ) -> Result<()> {
        let node = handle;

        match &node.data {
            NodeData::Element { ref attrs, .. } => {
                for attr in attrs.borrow().iter() {
                    if self.is_url_attribute(&attr.name.local) {
                        let url_str = attr.value.to_string();
                        if let Ok(url) = self.resolve_url(&url_str) {
                            let key = (url_str.clone(), url.to_string());
                            if seen.insert(key) {
                                references.push(UrlReference {
                                    original: url_str,
                                    resolved: url,
                                });
                            }
                        }
                    }
                }
            }
            NodeData::Text { ref contents } => {
                if let Some(urls_in_text) = self.extract_urls_from_text(&contents.borrow()) {
                    for url_str in urls_in_text {
                        if let Ok(url) = self.resolve_url(&url_str) {
                            let key = (url_str.clone(), url.to_string());
                            if seen.insert(key) {
                                references.push(UrlReference {
                                    original: url_str,
                                    resolved: url,
                                });
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        for child in node.children.borrow().iter() {
            self.walk_dom(child, references, seen)?;
        }

        Ok(())
    }

    fn is_url_attribute(&self, attr_name: &str) -> bool {
        matches!(
            attr_name,
            "src" | "href" | "data-src" | "data-original" | "poster" | "background" | "data-srcset" | "data-lazy-src"
        )
    }

    fn extract_urls_from_text(&self, text: &str) -> Option<Vec<String>> {
        let mut urls = Vec::new();
        
        // Extract URLs from CSS @import statements
        let import_regex = Regex::new(r#"@import\s+["']([^"']+)["']"#).ok()?;
        for cap in import_regex.captures_iter(text) {
            if let Some(url) = cap.get(1) {
                urls.push(url.as_str().to_string());
            }
        }

        // Extract URLs from CSS url() functions
        let url_regex = Regex::new(r#"url\(["']?([^"')]+)["']?\)"#).ok()?;
        for cap in url_regex.captures_iter(text) {
            if let Some(url) = cap.get(1) {
                urls.push(url.as_str().to_string());
            }
        }

        if urls.is_empty() {
            None
        } else {
            Some(urls)
        }
    }

    fn resolve_url(&self, url_str: &str) -> Result<Url> {
        if url_str.starts_with("data:") || url_str.starts_with("#") {
            return Err(anyhow!("Skipping data URL or fragment"));
        }

        if url_str.starts_with("//") {
            let scheme = self.base_url.scheme();
            Ok(Url::parse(&format!("{scheme}:{url_str}"))?)
        } else if url_str.starts_with('/') {
            // Absolute path
            let mut url = self.base_url.clone();
            url.set_path(url_str);
            Ok(url)
        } else if url_str.starts_with("http://") || url_str.starts_with("https://") {
            // Absolute URL
            Ok(Url::parse(url_str)?)
        } else {
            // Relative URL
            Ok(self.base_url.join(url_str)?)
        }
    }

    fn create_resource(&self, url: &Url) -> Result<Resource> {
        let resource_type = self.determine_resource_type(url);
        let local_path = self.generate_local_path(url, &resource_type)?;
        
        Ok(Resource {
            url: url.clone(),
            local_path,
            resource_type: resource_type.clone(),
            mime_type: self.guess_mime_type(url, &resource_type),
            size: None,
            downloaded: false,
        })
    }

    fn determine_resource_type(&self, url: &Url) -> ResourceType {
        let path = url.path();
        let extension = path.split('.').next_back().unwrap_or("").to_lowercase();

        // Check for explicit file extensions first
        match extension.as_str() {
            "html" | "htm" => ResourceType::HTML,
            "css" => ResourceType::CSS,
            "js" | "javascript" => ResourceType::JavaScript,
            "jpg" | "jpeg" | "png" | "gif" | "webp" | "svg" | "ico" | "bmp" | "tiff" | "tif" => ResourceType::Image,
            "mp4" | "webm" | "ogg" | "avi" | "mov" | "m4v" => ResourceType::Video,
            "pdf" => ResourceType::PDF,
            "woff" | "woff2" | "ttf" | "otf" | "eot" => ResourceType::Font,
            _ => {
                // For URLs without extensions, check if they look like HTML pages
                if self.looks_like_html_page(path) {
                    ResourceType::HTML
                } else {
                    ResourceType::Other
                }
            }
        }
    }

    fn looks_like_html_page(&self, path: &str) -> bool {
        // Skip empty paths and root
        if path.is_empty() || path == "/" {
            return false;
        }
        
        // Check if the path ends with a slash (directory-like)
        if path.ends_with('/') {
            return true;
        }
        
        // Check if the path looks like a content page (not a file)
        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        
        // If it's a single segment without extension, likely a page
        if segments.len() == 1 && !segments[0].contains('.') {
            return true;
        }
        
        // If it's multiple segments and the last one doesn't have an extension, likely a page
        if segments.len() > 1 {
            let last_segment = segments.last().unwrap_or(&"");
            if !last_segment.contains('.') && !last_segment.is_empty() {
                return true;
            }
        }
        
        false
    }

    fn generate_local_path(&self, url: &Url, resource_type: &ResourceType) -> Result<String> {
        let path = url.path();
        
        let subdirectory = match resource_type {
            ResourceType::HTML => "",  // HTML files maintain original structure
            ResourceType::CSS => "static/css",
            ResourceType::JavaScript => "static/js",
            ResourceType::Image => "static/images",
            ResourceType::Video => "static/video",
            ResourceType::PDF => "static/pdf",
            ResourceType::Font => "static/fonts",
            ResourceType::Other => "static/other",
        };

        // For HTML files, preserve the original directory structure
        if *resource_type == ResourceType::HTML {
            let path_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
            
            if path_segments.is_empty() || path == "/" {
                // Root page
                return Ok(format!("{}/index.html", self.output_dir));
            } else {
                // Create directory structure matching the original URL
                let dir_path = path_segments[..path_segments.len()-1].join("/");
                let filename = path_segments.last().unwrap_or(&"index");
                
                // Handle trailing slash (directory-like URLs)
                if path.ends_with('/') {
                    if dir_path.is_empty() {
                        return Ok(format!("{}/{}/index.html", self.output_dir, filename));
                    } else {
                        return Ok(format!("{}/{}/{}/index.html", self.output_dir, dir_path, filename));
                    }
                } else {
                    // Regular file path
                    if dir_path.is_empty() {
                        return Ok(format!("{}/{}.html", self.output_dir, filename));
                    } else {
                        return Ok(format!("{}/{}/{}.html", self.output_dir, dir_path, filename));
                    }
                }
            }
        }

        // For non-HTML resources, use the existing logic
        let filename = path.split('/').next_back().unwrap_or("unknown");
        let mut unique_filename = filename.to_string();
        if filename == "index.html" || filename.is_empty() {
            let host = url.host_str().unwrap_or("unknown");
            let path_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
            if path_segments.is_empty() {
                unique_filename = format!("{}.html", host);
            } else {
                unique_filename = format!("{}.html", path_segments.join("_"));
            }
        }

        Ok(format!("{}/{}/{}", self.output_dir, subdirectory, unique_filename))
    }

    fn guess_mime_type(&self, url: &Url, resource_type: &ResourceType) -> String {
        let path = url.path();
        let extension = path.split('.').next_back().unwrap_or("").to_lowercase();

        match extension.as_str() {
            "html" | "htm" => "text/html".to_string(),
            "css" => "text/css".to_string(),
            "js" => "application/javascript".to_string(),
            "jpg" | "jpeg" => "image/jpeg".to_string(),
            "png" => "image/png".to_string(),
            "gif" => "image/gif".to_string(),
            "webp" => "image/webp".to_string(),
            "svg" => "image/svg+xml".to_string(),
            "ico" => "image/x-icon".to_string(),
            "mp4" => "video/mp4".to_string(),
            "webm" => "video/webm".to_string(),
            "ogg" => "video/ogg".to_string(),
            "pdf" => "application/pdf".to_string(),
            "woff" => "font/woff".to_string(),
            "woff2" => "font/woff2".to_string(),
            "ttf" => "font/ttf".to_string(),
            _ => match resource_type {
                ResourceType::HTML => "text/html".to_string(),
                ResourceType::CSS => "text/css".to_string(),
                ResourceType::JavaScript => "application/javascript".to_string(),
                ResourceType::Image => "image/jpeg".to_string(),
                ResourceType::Video => "video/mp4".to_string(),
                ResourceType::PDF => "application/pdf".to_string(),
                ResourceType::Font => "font/woff".to_string(),
                ResourceType::Other => "application/octet-stream".to_string(),
            },
        }
    }

    fn replace_url_in_html(&self, html: &str, original_url: &str, local_path: &str) -> String {
        let relative_path = self.make_relative_path(local_path);
        html.replace(original_url, &relative_path)
    }

    fn make_relative_path(&self, local_path: &str) -> String {
        // Convert absolute path to relative path from the HTML file location
        if let Some(relative) = local_path.strip_prefix(&self.output_dir) {
            relative
                .strip_prefix('/')
                .unwrap_or(relative)
                .to_string()
        } else {
            local_path.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_resolution() {
        let base_url = Url::parse("https://example.com/page/").unwrap();
        let parser = HtmlParser::new(base_url, "./output".to_string());
        
        let relative_url = "image.jpg";
        let resolved = parser.resolve_url(relative_url).unwrap();
        assert_eq!(resolved.as_str(), "https://example.com/page/image.jpg");
    }

    #[test]
    fn test_resource_type_detection() {
        let base_url = Url::parse("https://example.com/").unwrap();
        let parser = HtmlParser::new(base_url, "./output".to_string());

        let image_url = Url::parse("https://example.com/image.png").unwrap();
        let resource_type = parser.determine_resource_type(&image_url);
        assert!(matches!(resource_type, ResourceType::Image));
    }

    #[test]
    fn test_protocol_relative_url_resolution() {
        let base_url = Url::parse("http://example.com/page/").unwrap();
        let parser = HtmlParser::new(base_url, "./output".to_string());

        let resolved = parser.resolve_url("//cdn.example.com/lib.js").unwrap();
        assert_eq!(resolved.as_str(), "http://cdn.example.com/lib.js");
    }

    #[test]
    fn test_extensionless_path_is_html() {
        let base_url = Url::parse("https://example.com/").unwrap();
        let parser = HtmlParser::new(base_url, "./output".to_string());

        let about = Url::parse("https://example.com/about").unwrap();
        assert_eq!(parser.determine_resource_type(&about), ResourceType::HTML);
    }

    #[test]
    fn test_absolute_path_resolution() {
        let base_url = Url::parse("https://example.com/page/").unwrap();
        let parser = HtmlParser::new(base_url, "./output".to_string());
        let resolved = parser.resolve_url("/assets/app.css").unwrap();
        assert_eq!(resolved.as_str(), "https://example.com/assets/app.css");
    }

    #[test]
    fn test_absolute_https_url_resolution() {
        let base_url = Url::parse("https://example.com/").unwrap();
        let parser = HtmlParser::new(base_url, "./output".to_string());
        let resolved = parser
            .resolve_url("https://cdn.example.com/lib.js")
            .unwrap();
        assert_eq!(resolved.as_str(), "https://cdn.example.com/lib.js");
    }

    #[test]
    fn test_skips_data_and_fragment_urls() {
        let base_url = Url::parse("https://example.com/").unwrap();
        let parser = HtmlParser::new(base_url, "./output".to_string());
        assert!(parser.resolve_url("#section").is_err());
        assert!(parser.resolve_url("data:image/png;base64,abc").is_err());
    }

    #[test]
    fn extracts_urls_from_inline_css() {
        let html = r#"<html><head><style>
            @import "imported.css";
            body { background: url(bg.png); }
        </style></head></html>"#;
        let base_url = Url::parse("https://example.com/").unwrap();
        let parser = HtmlParser::new(base_url, "./output".to_string());
        let (_, resources) = parser.parse_html(html).unwrap();
        assert!(
            resources
                .iter()
                .any(|r| r.url.path().ends_with("imported.css"))
        );
        assert!(
            resources.iter().any(|r| r.url.path().ends_with("bg.png"))
        );
    }

    #[test]
    fn guess_mime_type_falls_back_to_resource_type() {
        let base_url = Url::parse("https://example.com/").unwrap();
        let parser = HtmlParser::new(base_url, "./output".to_string());
        let url = Url::parse("https://example.com/no-extension").unwrap();
        assert_eq!(
            parser.guess_mime_type(&url, &ResourceType::JavaScript),
            "application/javascript"
        );
    }

    #[test]
    fn make_relative_path_strips_output_prefix() {
        let base_url = Url::parse("https://example.com/").unwrap();
        let parser = HtmlParser::new(base_url, "/output".to_string());
        let relative = parser.make_relative_path("/output/static/css/app.css");
        assert_eq!(relative, "static/css/app.css");
    }
} 