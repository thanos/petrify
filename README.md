# Petrify

[![crates.io](https://img.shields.io/crates/v/petrify.svg)](https://crates.io/crates/petrify)
[![docs.rs](https://docs.rs/petrify/badge.svg)](https://docs.rs/petrify)
[![CI](https://github.com/thanos/petrify/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/thanos/petrify/actions/workflows/ci.yml)
[![Code Quality](https://github.com/thanos/petrify/actions/workflows/code-quality.yml/badge.svg?branch=main)](https://github.com/thanos/petrify/actions/workflows/code-quality.yml)
[![Coverage Status](https://coveralls.io/repos/github/thanos/petrify/badge.svg?branch=main)](https://coveralls.io/github/thanos/petrify?branch=main)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![MSRV](https://img.shields.io/badge/MSRV-1.74-blue)](Cargo.toml)

Petrify is a command-line tool that downloads a website and produces a static copy suitable for offline browsing or hosting behind a web server. It crawls HTML pages on the target host, extracts linked assets, rewrites URLs to site-root paths, and saves everything to disk.

Petrify is similar in purpose to `wget --mirror`, with built-in link rewriting, concurrent downloads, optional image conversion to WebP, and discovery of SEO / social metadata images.

## What Petrify does

Given a starting URL, Petrify will:

1. Discover HTML pages on the same host by following links.
2. Download each page, parse it, and rewrite `href`, `src`, `style` `url(...)`, SEO meta images, and related references to local site-root paths.
3. Download referenced assets (CSS, JavaScript, images, fonts, video, PDF, and other selected types), including common off-host CDNs and object storage when linked from HTML.
4. Optionally convert JPEG, PNG, and GIF images to WebP during download (HTML links are updated to match).
5. Write a directory tree you can open locally or serve as a static site document root.

Petrify does not execute JavaScript or render pages in a browser. It works from the HTML and asset URLs present in the source.

## How it works

Petrify runs in four stages:

**Scan.** Starting from the URL you provide, Petrify downloads each HTML page on the same domain, extracts links and asset references, and builds a queue of pages to process and resources to download. A progress spinner shows scan status.

**Report.** Before downloading assets, Petrify prints a summary: target URL, output directory, page count, resource count, and active options.

**Process pages.** Each queued page is downloaded again, parsed, rewritten, and saved under the output directory. Pages are processed concurrently (see `--max-concurrent`).

**Download resources.** Assets are grouped by type, downloaded concurrently, and stored under `static/`. Each URL is downloaded at most once. Failed requests (including 404 responses) are logged with the URL.

### Domain scope

During the scan phase, only pages on the same host as the starting URL are followed. For example, if you start at `https://example.com/`, pages on `https://other.com/` linked from the site are not added to the crawl queue.

Asset URLs (images, CSS, JS, fonts, and so on) may be collected from the starting host and from external hosts when they appear in HTML attributes, inline CSS, or SEO metadata. See Limitations for notes on `--download-external`.

### Link rewriting

The HTML parser resolves relative URLs, absolute paths, absolute URLs, and protocol-relative URLs (`//cdn.example.com/...`). It reads URLs from:

- Common attributes (`src`, `href`, `data-src`, `poster`, and others)
- Inline CSS (`@import` and `url(...)` in `<style>` and `style="…"`)
- SEO / social metadata (`og:image`, `twitter:image`, and related `content` values)
- JSON-LD image-like fields (`image`, `thumbnailUrl`, `contentUrl`, `logo`, `photo`)

Rewritten links use **site-root paths** (for example `/static/images/logo.webp` and `/about/index.html`) so nested pages work when the output directory is served as a web server document root. URL fragments such as `#home` are preserved (`/page/index.html#home`).

## Installation

### From crates.io

```bash
cargo install petrify
```

### From GitHub Releases

Prebuilt binaries for Linux, macOS, and Windows are attached to each [GitHub release](https://github.com/thanos/petrify/releases).

### From source

```bash
git clone https://github.com/thanos/petrify.git
cd petrify
cargo install --path .
```

Requires Rust 1.74 or later.

## Quick start

```bash
# Download a site to ./petrified_site
petrify https://example.com

# Choose output directory and worker count
petrify https://example.com -o ./my_copy -m 4

# Limit how many pages are scanned and processed (useful for testing)
petrify https://example.com --max-pages 10

# Narrow the asset types (fonts are included by default)
petrify https://example.com --download-only html,css,js,images
```

### Serving the mirror

Point a static file server at the output directory (the document root):

```bash
cd petrified_site
python3 -m http.server 8000
# open http://127.0.0.1:8000/
```

Because links are site-root absolute (`/static/...`), nested pages such as `/artfestival/artist/name/` resolve assets correctly.

## Command reference

```
petrify [OPTIONS] <URL>
```

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `<URL>` | | Starting URL to crawl (required) | |
| `--output` | `-o` | Directory for the static copy | `./petrified_site` |
| `--max-concurrent` | `-m` | Number of concurrent download workers | CPU core count |
| `--download-only` | | Comma-separated list of resource types to download | `js,css,images,video,html,pdf,fonts` |
| `--max-pages` | | Maximum pages to scan and process (`0` = no limit) | `0` |
| `--depth` | `-d` | Maximum crawl depth | `unlimited` |
| `--convert-to-webp` | | Convert JPEG, PNG, and GIF images to WebP | `true` |
| `--webp-quality` | | WebP quality when lossy compression is used (1 to 100) | `75` |
| `--webp-lossless` | | Use lossless WebP compression instead of quality setting | `false` |
| `--download-external` | | Download resources from external hosts | `true` |
| `--ignore-robots` | `-i` | Ignore robots.txt restrictions | `true` |
| `--timeout` | | HTTP request timeout in seconds | `270` |
| `--help` | `-h` | Print help | |
| `--version` | `-V` | Print version | |

Run `petrify --help` for the full list.

### Resource types

The `--download-only` flag accepts these values:

| Value | File types |
|-------|------------|
| `html` | HTML pages (`.html`, `.htm`, extensionless paths treated as pages) |
| `css` | Stylesheets |
| `js` | JavaScript |
| `images` | Images (`.jpg`, `.jpeg`, `.png`, `.gif`, `.webp`, `.svg`, `.ico`, and related) |
| `video` | Video (`.mp4`, `.webm`, `.ogg`, `.avi`, `.mov`, and related) |
| `pdf` | PDF documents |
| `fonts` | Web fonts (`.woff`, `.woff2`, `.ttf`, `.otf`, `.eot`) |
| `other` | Everything else |

`fonts` is included in the default list. Meta / Open Graph / Twitter image URLs and JSON-LD image fields are discovered and rewritten when `images` is enabled.

To exclude fonts:

```bash
petrify https://example.com --download-only html,css,js,images
```

### Page limit

`--max-pages` limits how many pages are discovered during the scan and how many are written during processing. It does not cap the number of assets downloaded. Assets referenced by the pages that were scanned are still queued and downloaded according to `--download-only`.

### WebP conversion

When `--convert-to-webp` is enabled (the default), JPEG, PNG, and GIF images are converted to WebP before being saved. The file extension in the output directory changes to `.webp`, and HTML references are rewritten to match. Disable conversion to keep original image formats:

```bash
petrify https://example.com --convert-to-webp false
```

## Output layout

Petrify writes HTML pages under the output directory, preserving URL structure where possible. Assets are stored under `static/` by type.

```
petrified_site/
├── index.html              # https://example.com/
├── about.html              # https://example.com/about or about.html
├── blog/
│   └── post/
│       └── index.html      # https://example.com/blog/post/
└── static/
    ├── css/
    ├── js/
    ├── images/
    ├── video/
    ├── pdf/
    ├── fonts/
    └── other/
```

Path mapping examples:

| URL | Output file | Rewritten link |
|-----|-------------|----------------|
| `https://example.com/` | `index.html` | `/index.html` |
| `https://example.com/about` | `about.html` | `/about.html` |
| `https://example.com/docs/` | `docs/index.html` | `/docs/index.html` |
| `https://example.com/docs/#intro` | `docs/index.html` | `/docs/index.html#intro` |
| `https://example.com/static/app.css` | `static/css/app.css` | `/static/css/app.css` |
| `https://example.com/logo.png` | `static/images/logo.webp` (if conversion is on) | `/static/images/logo.webp` |

## Examples

### Documentation site

```bash
petrify https://docs.example.com \
  -o ./docs_offline \
  --download-only html,css,js,images,fonts \
  -m 4
```

### Images as WebP with higher quality

```bash
petrify https://example.com \
  -o ./site \
  --convert-to-webp \
  --webp-quality 85
```

### Small test run

```bash
petrify https://example.com \
  -o ./test_run \
  --max-pages 5 \
  --max-concurrent 2
```

## Logging

Petrify uses the standard `RUST_LOG` environment variable (via `env_logger`). To see warnings, including HTTP errors and 404 responses with the failing URL:

```bash
RUST_LOG=petrify=warn petrify https://example.com
```

For more detail during development:

```bash
RUST_LOG=petrify=info,petrify=debug petrify https://example.com
```

## Library

Petrify is also available as a Rust library. The main entry point is `petrify::engine::Petrifier`:

```rust
use petrify::config::Config;
use petrify::engine::Petrifier;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Config::new();
    config.url = "https://example.com".to_string();
    config.output = "./petrified_site".to_string();

    let mut petrifier = Petrifier::new(config).await?;
    petrifier.run().await?;
    Ok(())
}
```

API documentation is published at [docs.rs/petrify](https://docs.rs/petrify).

## Development

### Build and test

```bash
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --all
```

Integration tests use the fixture in `test_site/` and a local HTTP mock server ([wiremock](https://crates.io/crates/wiremock)).

### Coverage

CI runs tests with [tarpaulin](https://xd009642.github.io/tarpaulin/) and uploads results to Coveralls. To run coverage locally on Linux:

```bash
cargo install cargo-tarpaulin
cargo tarpaulin --all-targets --out Html --output-dir coverage
```

On macOS, [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov) is a practical alternative.

### Manual smoke test

```bash
./test_petrify.sh
```

This script builds the release binary and runs petrify against a few public URLs. It requires network access.

### Release

See [CHANGELOG.md](CHANGELOG.md) for release notes. Tagging `v*` on the default branch triggers GitHub Actions to build binaries, create a GitHub release, and publish to crates.io (requires the `CRATES_IO_TOKEN` repository secret).

```bash
# After merging release-ready changes to main:
# 1. Ensure version in Cargo.toml matches the tag (e.g. 0.2.0)
# 2. Commit CHANGELOG + version bump
# 3. Tag and push
git tag -a v0.2.0 -m "Release 0.2.0"
git push origin v0.2.0
```

## Limitations

Petrify produces a static snapshot. It has the following constraints:

- **No JavaScript rendering.** Content loaded or modified only by client-side JavaScript after page load will not appear in the output. Single-page applications may not archive completely.
- **Same-host crawling.** The scan phase only follows links to pages on the same host as the starting URL.
- **Depth not enforced.** `--depth` is accepted on the command line but is not currently applied during crawling.
- **External resources.** `--download-external` is accepted but not fully enforced as a hard gate. Off-host assets referenced in HTML may still be queued depending on how they are linked.
- **robots.txt.** `--ignore-robots` is accepted but robots.txt is not fetched or checked. Use Petrify only on sites you are permitted to copy.
- **HTTP errors.** Failed downloads are logged and skipped. The run continues; the output may contain broken references where a resource could not be fetched.
- **CSS-linked fonts.** Fonts referenced only from downloaded CSS files (not from HTML) are not yet crawled as a second pass.
- **Scale.** Large sites require significant time, bandwidth, and disk space. Use `--max-pages` to test on a subset first.

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for the full history. Latest release: **0.2.0**.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE).
