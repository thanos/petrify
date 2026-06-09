//! Petrify — turn live websites into static offline copies.
//!
//! The library powers the `petrify` CLI. It discovers pages, rewrites links to
//! local paths, and downloads referenced assets (HTML, CSS, JS, images, etc.).

pub mod config;
pub mod engine;
pub mod html_parser;
pub mod types;
