//! Petrify — turn live websites into static offline copies.
//!
//! The library powers the `petrify` CLI. It discovers pages on a starting host,
//! downloads referenced assets, rewrites links to site-root paths suitable for
//! hosting behind a web server, and optionally converts images to WebP.
//!
//! # Quick example
//!
//! ```no_run
//! use petrify::config::Config;
//! use petrify::engine::Petrifier;
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let mut config = Config::new();
//! config.url = "https://example.com".to_string();
//! config.output = "./petrified_site".to_string();
//!
//! let mut petrifier = Petrifier::new(config).await?;
//! petrifier.run().await?;
//! # Ok(())
//! # }
//! ```
//!
//! See the crate README and [CHANGELOG](https://github.com/thanos/petrify/blob/main/CHANGELOG.md)
//! for CLI options and release notes.

pub mod config;
pub mod engine;
pub mod html_parser;
pub mod paths;
pub mod types;
