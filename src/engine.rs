use crate::config::Config;
use crate::html_parser::HtmlParser;
use crate::paths::{normalize_output_dir, update_path_to_webp};
use crate::types::{DownloadStats, Resource, ResourceType, WorkQueue};
use anyhow::{anyhow, Result};
use colored::*;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use log::{error, info, warn};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::Semaphore;
use url::Url;

pub struct Petrifier {
    config: Config,
    work_queue: Arc<Mutex<WorkQueue>>,
    stats: Arc<Mutex<DownloadStats>>,
    downloaded_resources: Arc<Mutex<HashMap<String, Resource>>>,
    multi_progress: MultiProgress,
}

impl Petrifier {
    pub async fn new(mut config: Config) -> Result<Self> {
        config.output = normalize_output_dir(&config.output);

        // Create output directories
        Self::create_output_directories(&config.output)?;

        let multi_progress = MultiProgress::new();

        Ok(Self {
            config,
            work_queue: Arc::new(Mutex::new(WorkQueue::new())),
            stats: Arc::new(Mutex::new(DownloadStats::new())),
            downloaded_resources: Arc::new(Mutex::new(HashMap::new())),
            multi_progress,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        info!("Starting petrify process");

        // Parse the initial URL
        let base_url = Url::parse(&self.config.url)?;

        // Add the initial page to the work queue
        {
            let mut queue = self.work_queue.lock().unwrap();
            queue.add_page(base_url.clone());
        }

        // Scan the site to discover all pages first
        self.scan_site(&base_url).await?;

        // Show initial report
        self.show_initial_report().await?;

        // Process pages with concurrent workers
        self.process_pages_concurrently().await?;

        // Download all resources
        self.download_all_resources().await?;

        // Show final report
        self.show_final_report().await?;

        Ok(())
    }

    async fn scan_site(&self, base_url: &Url) -> Result<()> {
        info!("Scanning site to discover all pages...");

        let mut visited = HashMap::new();
        let mut to_visit = vec![base_url.clone()];
        let mut discovered_pages = 0;
        let mut discovered_resources = 0;

        // Create progress bar for scanning
        let scan_progress = self.multi_progress.add(ProgressBar::new_spinner());
        scan_progress.set_message("🔍 Scanning site for pages and resources...");

        while let Some(url) = to_visit.pop() {
            let normalized = self.normalize_url(&url);
            if visited.contains_key(&normalized) {
                continue;
            }

            visited.insert(normalized.clone(), true);
            discovered_pages += 1;

            scan_progress.set_message(format!(
                "🔍 Scanning page {}: {}",
                discovered_pages,
                url.path()
            ));

            if self.config.is_page_limit_reached(discovered_pages) {
                scan_progress.set_message(format!(
                    "🛑 Page limit reached ({}) - stopping discovery",
                    self.config.max_pages
                ));
                break;
            }

            if !self.is_same_domain(&url, base_url) {
                continue;
            }

            match self.download_page(&url).await {
                Ok(html_content) => {
                    let parser = HtmlParser::new(
                        url.clone(),
                        self.config.output.clone(),
                        self.config.convert_to_webp,
                    );
                    if let Ok((_, resources)) = parser.parse_html(&html_content) {
                        for resource in &resources {
                            if resource.resource_type == ResourceType::HTML {
                                let page_url = resource.url.clone();
                                if !visited.contains_key(&self.normalize_url(&page_url)) {
                                    to_visit.push(page_url.clone());
                                    let mut queue = self.work_queue.lock().unwrap();
                                    queue.add_page(page_url);
                                }
                            }
                        }

                        {
                            let mut queue = self.work_queue.lock().unwrap();
                            for resource in &resources {
                                if resource.resource_type != ResourceType::HTML {
                                    queue.add_resource(resource.url.clone());
                                }
                            }
                            discovered_resources = queue.resources.len();
                        }

                        scan_progress.set_message(format!(
                            "🔍 Found {} pages to crawl, {} resources to download - Currently scanning: {}",
                            discovered_pages,
                            discovered_resources,
                            url.path()
                        ));
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to download page during scan {}: {}",
                        url.as_str(),
                        e
                    );
                }
            }
        }

        scan_progress.finish_with_message(format!(
            "✅ Site scanning completed! Found {} pages to crawl and {} resources to download",
            discovered_pages, discovered_resources
        ));

        info!(
            "Site scanning completed - {} pages to crawl, {} resources to download",
            discovered_pages, discovered_resources
        );
        Ok(())
    }

    async fn process_pages_concurrently(&self) -> Result<()> {
        let semaphore = Arc::new(Semaphore::new(self.config.max_concurrent));
        let mut handles = Vec::new();

        // Get all pages to process
        let mut pages = {
            let queue = self.work_queue.lock().unwrap();
            queue.pages.clone()
        };

        // Apply page limit if set
        if self.config.should_limit_pages() {
            pages.truncate(self.config.max_pages);
            info!(
                "Page limit applied: processing only {} pages out of {} discovered",
                pages.len(),
                self.config.max_pages
            );
        }

        // Update stats
        {
            let mut stats = self.stats.lock().unwrap();
            stats.total_pages = pages.len();
        }

        info!(
            "Processing {} pages with {} concurrent workers",
            pages.len(),
            self.config.max_concurrent
        );

        for page_url in pages {
            let permit = semaphore.clone().acquire_owned().await?;
            let work_queue = Arc::clone(&self.work_queue);
            let stats = Arc::clone(&self.stats);
            let config = self.config.clone();
            let output_dir = self.config.output.clone();

            let handle = tokio::spawn(async move {
                let _permit = permit;
                Self::process_single_page(page_url, work_queue, stats, config, output_dir).await
            });

            handles.push(handle);
        }

        // Wait for all pages to be processed
        for handle in handles {
            if let Err(e) = handle.await? {
                error!("Page processing failed: {}", e);
            }
        }

        Ok(())
    }

    async fn process_single_page(
        page_url: Url,
        work_queue: Arc<Mutex<WorkQueue>>,
        stats: Arc<Mutex<DownloadStats>>,
        config: Config,
        output_dir: String,
    ) -> Result<()> {
        // Download the page
        let html_content = Self::download_page_static(&page_url, &config).await?;

        // Parse HTML and extract resources
        let html_content_str = String::from_utf8(html_content)?;
        let parser = HtmlParser::new(page_url.clone(), output_dir.clone(), config.convert_to_webp);
        let (modified_html, resources) = parser.parse_html(&html_content_str)?;

        // Save the modified HTML
        let page_path = Self::generate_page_path(&page_url, &output_dir)?;
        Self::ensure_directory_exists(&page_path)?;
        fs::write(&page_path, modified_html)?;

        // Add resources to the work queue
        {
            let mut queue = work_queue.lock().unwrap();
            for resource in resources {
                queue.add_resource(resource.url);
            }
        }

        // Update stats
        {
            let mut stats = stats.lock().unwrap();
            stats.processed_pages += 1;
        }

        info!("Processed page: {}", page_url);
        Ok(())
    }

    async fn download_all_resources(&self) -> Result<()> {
        info!("Downloading all resources...");

        let resources = {
            let queue = self.work_queue.lock().unwrap();
            queue.resources.clone()
        };

        // Update stats
        {
            let mut stats = self.stats.lock().unwrap();
            stats.total_resources = resources.len();
        }

        // Group resources by type
        let mut resources_by_type: HashMap<ResourceType, Vec<Url>> = HashMap::new();
        for url in resources {
            let resource_type = Self::determine_resource_type(&url);
            resources_by_type
                .entry(resource_type)
                .or_default()
                .push(url);
        }

        // Download resources by type with progress bars
        for (resource_type, urls) in resources_by_type {
            if !self.config.should_download_type(&resource_type) {
                continue;
            }

            self.download_resources_by_type(resource_type, urls).await?;
        }

        Ok(())
    }

    async fn download_resources_by_type(
        &self,
        resource_type: ResourceType,
        urls: Vec<Url>,
    ) -> Result<()> {
        let type_name = format!("{:?}", resource_type);
        info!("Downloading {} {} resources", urls.len(), type_name);

        let progress_bar = self.multi_progress.add(ProgressBar::new(urls.len() as u64));
        progress_bar.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );

        let semaphore = Arc::new(Semaphore::new(self.config.max_concurrent));
        let mut handles = Vec::new();

        for url in &urls {
            let permit = semaphore.clone().acquire_owned().await?;
            let url_clone = url.clone();
            let resource_type_clone = resource_type.clone();
            let config = self.config.clone();
            let output_dir = self.config.output.clone();
            let progress_bar = progress_bar.clone();
            let downloaded_resources = Arc::clone(&self.downloaded_resources);
            let stats = Arc::clone(&self.stats);

            let handle = tokio::spawn(async move {
                let _permit = permit;
                Self::download_single_resource(
                    url_clone,
                    resource_type_clone,
                    config,
                    output_dir,
                    progress_bar,
                    downloaded_resources,
                    stats,
                )
                .await
            });

            handles.push(handle);
        }

        // Wait for all resources to be downloaded
        for handle in handles {
            if let Err(e) = handle.await? {
                error!("Resource download failed: {}", e);
            }
        }

        progress_bar.finish_with_message(format!(
            "{} {} resources downloaded",
            urls.len(),
            type_name
        ));
        Ok(())
    }

    async fn download_single_resource(
        url: Url,
        resource_type: ResourceType,
        config: Config,
        output_dir: String,
        progress_bar: ProgressBar,
        downloaded_resources: Arc<Mutex<HashMap<String, Resource>>>,
        stats: Arc<Mutex<DownloadStats>>,
    ) -> Result<()> {
        let normalized_url = Self::normalize_url_static(&url);

        // Check if already downloaded
        {
            let downloaded = downloaded_resources.lock().unwrap();
            if downloaded.contains_key(&normalized_url) {
                progress_bar.inc(1);
                return Ok(());
            }
        }

        let content = Self::download_page_static(&url, &config).await?;

        // Generate local path
        let local_path = Self::generate_resource_path(&url, &resource_type, &output_dir)?;
        Self::ensure_directory_exists(&local_path)?;

        // Process based on resource type
        let (final_content, final_local_path) = match resource_type {
            ResourceType::Image if config.convert_to_webp => {
                let webp_content = Self::convert_image_to_webp(&content, &config)?;
                // Update path to .webp extension
                let webp_path = update_path_to_webp(&local_path);
                (webp_content, webp_path)
            }
            _ => (content.clone(), local_path.clone()),
        };

        // Save the resource
        fs::write(&final_local_path, &final_content)?;

        // Create resource record
        let resource = Resource {
            url: url.clone(),
            local_path: final_local_path.clone(),
            resource_type: resource_type.clone(),
            mime_type: Self::guess_mime_type(&url, &resource_type),
            size: Some(final_content.len() as u64),
            downloaded: true,
        };

        // Store in downloaded resources
        {
            let mut downloaded = downloaded_resources.lock().unwrap();
            downloaded.insert(normalized_url, resource);
        }

        // Update stats
        {
            let mut stats = stats.lock().unwrap();
            stats.downloaded_resources += 1;
            stats.total_size += final_content.len() as u64;
        }

        progress_bar.inc(1);
        progress_bar.set_message(format!(
            "Downloaded: {}",
            url.path().split('/').next_back().unwrap_or("unknown")
        ));

        Ok(())
    }

    async fn download_page(&self, url: &Url) -> Result<String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(self.config.timeout))
            .build()?;

        let response = client.get(url.as_str()).send().await?;
        let status = response.status();
        if !status.is_success() {
            if status == reqwest::StatusCode::NOT_FOUND {
                warn!("404 Not Found: {}", url.as_str());
            } else {
                warn!("HTTP {}: {}", status, url.as_str());
            }
            return Err(anyhow!("HTTP error {} for {}", status, url.as_str()));
        }

        let content = response.text().await?;
        Ok(content)
    }

    async fn download_page_static(url: &Url, config: &Config) -> Result<Vec<u8>> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout))
            .build()?;

        let response = client.get(url.as_str()).send().await?;
        let status = response.status();
        if !status.is_success() {
            if status == reqwest::StatusCode::NOT_FOUND {
                warn!("404 Not Found: {}", url.as_str());
            } else {
                warn!("HTTP {}: {}", status, url.as_str());
            }
            return Err(anyhow!("HTTP error {} for {}", status, url.as_str()));
        }

        let content = response.bytes().await?;
        Ok(content.to_vec())
    }

    fn convert_image_to_webp(image_data: &[u8], config: &Config) -> Result<Vec<u8>> {
        let img = image::load_from_memory(image_data)?;

        let encoder =
            webp::Encoder::from_image(&img).map_err(|e| anyhow!("WebP encoder error: {}", e))?;
        let webp_data = if config.webp_lossless {
            encoder.encode_lossless()
        } else {
            encoder.encode(config.webp_quality as f32)
        };

        Ok(webp_data.to_vec())
    }

    fn generate_page_path(url: &Url, output_dir: &str) -> Result<String> {
        let path = url.path();

        if path.is_empty() || path == "/" {
            // Root page
            return Ok(format!("{}/index.html", output_dir));
        }

        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        if segments.is_empty() {
            return Ok(format!("{}/index.html", output_dir));
        }

        // Create directory structure matching the original URL
        if segments.len() == 1 {
            let filename = segments[0];
            if path.ends_with('/') {
                Ok(format!("{}/{}/index.html", output_dir, filename))
            } else if filename.ends_with(".html") || filename.ends_with(".htm") {
                Ok(format!("{}/{}", output_dir, filename))
            } else {
                Ok(format!("{}/{}.html", output_dir, filename))
            }
        } else {
            let dir_path = segments[..segments.len() - 1].join("/");
            let filename = segments.last().unwrap();

            if path.ends_with('/') {
                Ok(format!(
                    "{}/{}/{}/index.html",
                    output_dir, dir_path, filename
                ))
            } else if filename.ends_with(".html") || filename.ends_with(".htm") {
                Ok(format!("{}/{}/{}", output_dir, dir_path, filename))
            } else {
                Ok(format!("{}/{}/{}.html", output_dir, dir_path, filename))
            }
        }
    }

    fn generate_resource_path(
        url: &Url,
        resource_type: &ResourceType,
        output_dir: &str,
    ) -> Result<String> {
        let path = url.path();
        let filename = path.split('/').next_back().unwrap_or("unknown");

        let subdirectory = match resource_type {
            ResourceType::CSS => "static/css",
            ResourceType::JavaScript => "static/js",
            ResourceType::Image => "static/images",
            ResourceType::Video => "static/video",
            ResourceType::PDF => "static/pdf",
            ResourceType::Font => "static/fonts",
            _ => "static/other",
        };

        Ok(format!("{}/{}/{}", output_dir, subdirectory, filename))
    }

    fn ensure_directory_exists(file_path: &str) -> Result<()> {
        if let Some(parent) = Path::new(file_path).parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(())
    }

    fn create_output_directories(output_dir: &str) -> Result<()> {
        let directories = [
            "static/css",
            "static/js",
            "static/images",
            "static/video",
            "static/pdf",
            "static/fonts",
            "static/other",
        ];

        for dir in &directories {
            fs::create_dir_all(format!("{}/{}", output_dir, dir))?;
        }

        Ok(())
    }

    fn normalize_url(&self, url: &Url) -> String {
        let mut normalized = url.clone();
        normalized.set_fragment(None);
        normalized.set_query(None);
        normalized.to_string()
    }

    fn normalize_url_static(url: &Url) -> String {
        let mut normalized = url.clone();
        normalized.set_fragment(None);
        normalized.set_query(None);
        normalized.to_string()
    }

    fn is_same_domain(&self, url: &Url, base_url: &Url) -> bool {
        url.host_str() == base_url.host_str()
    }

    fn determine_resource_type(url: &Url) -> ResourceType {
        let path = url.path();
        let extension = path.split('.').next_back().unwrap_or("").to_lowercase();

        match extension.as_str() {
            "html" | "htm" => ResourceType::HTML,
            "css" => ResourceType::CSS,
            "js" => ResourceType::JavaScript,
            "jpg" | "jpeg" | "png" | "gif" | "webp" | "svg" | "ico" => ResourceType::Image,
            "mp4" | "webm" | "ogg" | "avi" | "mov" => ResourceType::Video,
            "pdf" => ResourceType::PDF,
            "woff" | "woff2" | "ttf" | "otf" | "eot" => ResourceType::Font,
            _ => ResourceType::Other,
        }
    }

    fn guess_mime_type(url: &Url, resource_type: &ResourceType) -> String {
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

    async fn show_initial_report(&self) -> Result<()> {
        let queue = self.work_queue.lock().unwrap();
        let _stats = self.stats.lock().unwrap();

        println!("\n{}", "=".repeat(60).blue());
        println!("{}", "PETRIFY INITIAL REPORT".bold().blue());
        println!("{}", "=".repeat(60).blue());
        println!("Target URL: {}", self.config.url.green());
        println!("Output Directory: {}", self.config.output.green());
        println!(
            "Discovered Pages: {}",
            queue.pages.len().to_string().yellow()
        );
        println!(
            "Discovered Resources: {}",
            queue.resources.len().to_string().yellow()
        );
        println!(
            "Max Concurrent Workers: {}",
            self.config.max_concurrent.to_string().cyan()
        );
        if self.config.should_limit_pages() {
            println!("Page Limit: {} pages (for testing)", self.config.max_pages);
        }
        println!(
            "Download Types: {}",
            self.config.download_only.join(", ").cyan()
        );
        println!(
            "WebP Conversion: {}",
            if self.config.convert_to_webp {
                "Enabled".green()
            } else {
                "Disabled".red()
            }
        );
        println!("Output Structure: HTML files in root, resources in static/ subdirectories");
        println!("{}", "=".repeat(60).blue());
        println!();

        Ok(())
    }

    async fn show_final_report(&self) -> Result<()> {
        let stats = self.stats.lock().unwrap();
        let _downloaded = self.downloaded_resources.lock().unwrap();

        println!("\n{}", "=".repeat(60).green());
        println!("{}", "PETRIFY COMPLETED".bold().green());
        println!("{}", "=".repeat(60).green());
        println!(
            "Total Pages Processed: {}",
            stats.processed_pages.to_string().green()
        );
        println!(
            "Total Resources Downloaded: {}",
            stats.downloaded_resources.to_string().green()
        );
        println!(
            "Total Size Downloaded: {} bytes",
            stats.total_size.to_string().green()
        );
        println!("Elapsed Time: {:?}", stats.elapsed_time());
        println!("Output Directory: {}", self.config.output.green());
        println!("{}", "=".repeat(60).green());
        println!();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn tiny_png() -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 0, 0, 255]));
        let mut buf = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .unwrap();
        buf
    }

    fn test_config() -> Config {
        Config::new()
    }

    #[test]
    fn generate_page_path_root() {
        let url = Url::parse("https://example.com/").unwrap();
        let path = Petrifier::generate_page_path(&url, "/out").unwrap();
        assert_eq!(path, "/out/index.html");
    }

    #[test]
    fn generate_page_path_single_segment() {
        let url = Url::parse("https://example.com/about").unwrap();
        let path = Petrifier::generate_page_path(&url, "/out").unwrap();
        assert_eq!(path, "/out/about.html");
    }

    #[test]
    fn generate_page_path_preserves_html_extension() {
        let url = Url::parse("https://example.com/about.html").unwrap();
        let path = Petrifier::generate_page_path(&url, "/out").unwrap();
        assert_eq!(path, "/out/about.html");
    }

    #[test]
    fn generate_page_path_nested_directory() {
        let url = Url::parse("https://example.com/blog/post/").unwrap();
        let path = Petrifier::generate_page_path(&url, "/out").unwrap();
        assert_eq!(path, "/out/blog/post/index.html");
    }

    #[test]
    fn generate_resource_path_places_assets_in_static_dirs() {
        let css = Url::parse("https://example.com/static/app.css").unwrap();
        let js = Url::parse("https://example.com/app.js").unwrap();
        let img = Url::parse("https://example.com/photo.png").unwrap();

        assert_eq!(
            Petrifier::generate_resource_path(&css, &ResourceType::CSS, "/out").unwrap(),
            "/out/static/css/app.css"
        );
        assert_eq!(
            Petrifier::generate_resource_path(&js, &ResourceType::JavaScript, "/out").unwrap(),
            "/out/static/js/app.js"
        );
        assert_eq!(
            Petrifier::generate_resource_path(&img, &ResourceType::Image, "/out").unwrap(),
            "/out/static/images/photo.png"
        );
    }

    #[test]
    fn determine_resource_type_from_extension() {
        assert_eq!(
            Petrifier::determine_resource_type(&Url::parse("https://example.com/a.js").unwrap()),
            ResourceType::JavaScript
        );
        assert_eq!(
            Petrifier::determine_resource_type(&Url::parse("https://example.com/a.pdf").unwrap()),
            ResourceType::PDF
        );
    }

    #[test]
    fn normalize_url_strips_query_and_fragment() {
        let url = Url::parse("https://example.com/page?q=1#frag").unwrap();
        assert_eq!(
            Petrifier::normalize_url_static(&url),
            "https://example.com/page"
        );
    }

    #[test]
    fn guess_mime_type_from_extension() {
        let url = Url::parse("https://example.com/app.js").unwrap();
        assert_eq!(
            Petrifier::guess_mime_type(&url, &ResourceType::JavaScript),
            "application/javascript"
        );
    }

    #[test]
    fn update_path_to_webp_changes_extension() {
        let updated = crate::paths::update_path_to_webp("/out/static/images/photo.png");
        assert_eq!(updated, "/out/static/images/photo.webp");
    }

    #[test]
    fn convert_image_to_webp_produces_bytes() {
        let config = test_config();
        let webp = Petrifier::convert_image_to_webp(&tiny_png(), &config).unwrap();
        assert!(!webp.is_empty());
        assert_ne!(webp, tiny_png());
    }

    #[test]
    fn create_output_directories_makes_static_subdirs() {
        let dir = TempDir::new().unwrap();
        let output = dir.path().join("site");
        Petrifier::create_output_directories(&output.to_string_lossy()).unwrap();
        assert!(output.join("static/css").is_dir());
        assert!(output.join("static/js").is_dir());
        assert!(output.join("static/images").is_dir());
    }

    #[test]
    fn ensure_directory_exists_creates_parents() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("a/b/c.txt");
        Petrifier::ensure_directory_exists(&file.to_string_lossy()).unwrap();
        assert!(file.parent().unwrap().is_dir());
    }

    #[tokio::test]
    async fn petrifier_new_initializes_output_layout() {
        let dir = TempDir::new().unwrap();
        let mut config = test_config();
        config.output = dir.path().join("out").to_string_lossy().to_string();
        let _ = Petrifier::new(config).await.unwrap();
        assert!(dir.path().join("out/static/css").is_dir());
    }

    #[tokio::test]
    async fn download_page_static_fetches_body() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/hello"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"petrified"))
            .mount(&server)
            .await;

        let config = test_config();
        let url = Url::parse(&format!("{}/hello", server.uri())).unwrap();
        let body = Petrifier::download_page_static(&url, &config)
            .await
            .unwrap();
        assert_eq!(body, b"petrified");
    }

    #[tokio::test]
    async fn download_page_static_errors_on_404() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/missing"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let config = test_config();
        let url = Url::parse(&format!("{}/missing", server.uri())).unwrap();
        let err = Petrifier::download_page_static(&url, &config)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("404"));
    }

    #[tokio::test]
    async fn download_page_fetches_text() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_string("<html></html>"))
            .mount(&server)
            .await;

        let dir = TempDir::new().unwrap();
        let mut config = test_config();
        config.output = dir.path().to_string_lossy().to_string();
        config.timeout = 5;
        let petrifier = Petrifier::new(config).await.unwrap();
        let url = Url::parse(&format!("{}/", server.uri())).unwrap();
        let html = petrifier.download_page(&url).await.unwrap();
        assert_eq!(html, "<html></html>");
    }

    #[test]
    fn determine_resource_type_covers_media_and_fonts() {
        assert_eq!(
            Petrifier::determine_resource_type(&Url::parse("https://example.com/v.mp4").unwrap()),
            ResourceType::Video
        );
        assert_eq!(
            Petrifier::determine_resource_type(&Url::parse("https://example.com/f.woff2").unwrap()),
            ResourceType::Font
        );
        assert_eq!(
            Petrifier::determine_resource_type(&Url::parse("https://example.com/unknown").unwrap()),
            ResourceType::Other
        );
    }

    #[test]
    fn guess_mime_type_uses_fallback_for_unknown_extension() {
        let url = Url::parse("https://example.com/file").unwrap();
        assert_eq!(
            Petrifier::guess_mime_type(&url, &ResourceType::CSS),
            "text/css"
        );
    }

    #[test]
    fn convert_image_lossless_webp() {
        let mut config = test_config();
        config.webp_lossless = true;
        let webp = Petrifier::convert_image_to_webp(&tiny_png(), &config).unwrap();
        assert!(!webp.is_empty());
    }

    #[test]
    fn normalize_url_instance_method() {
        let dir = TempDir::new().unwrap();
        let mut config = test_config();
        config.output = dir.path().to_string_lossy().to_string();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let petrifier = rt.block_on(Petrifier::new(config)).unwrap();
        let url = Url::parse("https://example.com/x?y=1#z").unwrap();
        assert_eq!(petrifier.normalize_url(&url), "https://example.com/x");
    }

    #[test]
    fn is_same_domain_matches_host() {
        let dir = TempDir::new().unwrap();
        let mut config = test_config();
        config.output = dir.path().to_string_lossy().to_string();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let petrifier = rt.block_on(Petrifier::new(config)).unwrap();
        let base = Url::parse("https://example.com/").unwrap();
        let same = Url::parse("https://example.com/other").unwrap();
        let other = Url::parse("https://other.com/").unwrap();
        assert!(petrifier.is_same_domain(&same, &base));
        assert!(!petrifier.is_same_domain(&other, &base));
    }
}
