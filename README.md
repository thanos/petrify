# Petrify

Petrify is a command-line tool that downloads a website and produces a static copy suitable for offline browsing. It crawls HTML pages on the target host, extracts linked assets, rewrites URLs to local paths, and saves everything to disk.

Petrify is similar in purpose to `wget --mirror`, with built-in link rewriting, concurrent downloads, and optional image conversion to WebP.

## What Petrify does

Given a starting URL, Petrify will:

1. Discover HTML pages on the same host by following links.
2. Download each page, parse it, and rewrite `href`, `src`, and related attributes to point at local files.
3. Download referenced assets (CSS, JavaScript, images, and other file types you select).
4. Optionally convert JPEG, PNG, and GIF images to WebP during download.
5. Write a directory tree you can open in a browser without a network connection.

Petrify does not execute JavaScript or render pages in a browser. It works from the HTML and asset URLs present in the source.

## How it works

Petrify runs in four stages:

**Scan.** Starting from the URL you provide, Petrify downloads each HTML page on the same domain, extracts links and asset references, and builds a queue of pages to process and resources to download. A progress spinner shows scan status.

**Report.** Before downloading assets, Petrify prints a summary: target URL, output directory, page count, resource count, and active options.

**Process pages.** Each queued page is downloaded again, parsed, rewritten, and saved under the output directory. Pages are processed concurrently (see `--max-concurrent`).

**Download resources.** Assets are grouped by type, downloaded concurrently, and stored under `static/`. Each URL is downloaded at most once. Failed requests (including 404 responses) are logged with the URL.

### Domain scope

During the scan phase, only pages on the same host as the starting URL are followed. For example, if you start at `https://example.com/`, pages on `https://other.com/` linked from the site are not added to the crawl queue.

Asset URLs on the same host are always collected. Whether off-host assets are downloaded depends on your `--download-only` settings; see Limitations for notes on `--download-external`.

### Link rewriting

The HTML parser resolves relative URLs, absolute paths, absolute URLs, and protocol-relative URLs (`//cdn.example.com/...`). It reads URLs from common attributes (`src`, `href`, `data-src`, `poster`, and others) and from inline CSS (`@import` and `url(...)`).

Original attribute values in the HTML are replaced with paths relative to the output directory so that saved pages load assets locally.

## Installation

### From crates.io

```bash
cargo install petrify
```

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

# Download only HTML, CSS, JavaScript, and images
petrify https://example.com --download-only html,css,js,images
```

Open the output directory in a browser. The entry point is typically `index.html` at the root of the output path.

## Command reference

```
petrify [OPTIONS] <URL>
```

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `<URL>` | | Starting URL to crawl (required) | |
| `--output` | `-o` | Directory for the static copy | `./petrified_site` |
| `--max-concurrent` | `-m` | Number of concurrent download workers | CPU core count |
| `--download-only` | | Comma-separated list of resource types to download | `js,css,images,video,html,pdf` |
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

`fonts` is not included in the default list. Add it explicitly if you need web fonts:

```bash
petrify https://example.com --download-only html,css,js,images,fonts
```

### Page limit

`--max-pages` limits how many pages are discovered during the scan and how many are written during processing. It does not cap the number of assets downloaded. Assets referenced by the pages that were scanned are still queued and downloaded according to `--download-only`.

### WebP conversion

When `--convert-to-webp` is enabled (the default), JPEG, PNG, and GIF images are converted to WebP before being saved. The file extension in the output directory changes to `.webp`. Disable conversion to keep original image formats:

```bash
petrify https://example.com --convert-to-webp false
```

## Output layout

Petrify writes HTML pages at the root of the output directory, preserving URL structure where possible. Assets are stored under `static/` by type.

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

| URL | Output file |
|-----|-------------|
| `https://example.com/` | `index.html` |
| `https://example.com/about` | `about.html` |
| `https://example.com/about.html` | `about.html` |
| `https://example.com/docs/` | `docs/index.html` |
| `https://example.com/static/app.css` | `static/css/app.css` |
| `https://example.com/logo.png` | `static/images/logo.png` (or `logo.webp` if conversion is on) |

## Examples

### Documentation site

```bash
petrify https://docs.example.com \
  -o ./docs_offline \
  --download-only html,css,js,images \
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

Integration tests use the fixture in `test_site/`. End-to-end tests use a local HTTP mock server (wiremock).

### Coverage

CI runs tests with [tarpaulin](https://github.com/xd009642/tarpaulin) and uploads results to Coveralls. To run coverage locally on Linux:

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

## Limitations

Petrify produces a static snapshot. It has the following constraints:

- **No JavaScript rendering.** Content loaded or modified only by client-side JavaScript after page load will not appear in the output. Single-page applications may not archive completely.
- **Same-host crawling.** The scan phase only follows links to pages on the same host as the starting URL.
- **Depth not enforced.** `--depth` is accepted on the command line but is not currently applied during crawling.
- **External resources.** `--download-external` is accepted but not fully enforced. Off-host assets referenced in HTML may still be queued depending on how they are linked.
- **robots.txt.** `--ignore-robots` is accepted but robots.txt is not fetched or checked. Use Petrify only on sites you are permitted to copy.
- **HTTP errors.** Failed downloads are logged and skipped. The run continues; the output may contain broken references where a resource could not be fetched.
- **Scale.** Large sites require significant time, bandwidth, and disk space. Use `--max-pages` to test on a subset first.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE).
