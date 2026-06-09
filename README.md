# Petrify

Petrify turns live websites into static offline copies. Crawl a site, download its assets, rewrite links for local browsing, and optionally convert images to WebP.

## Features

- Recursive site discovery and static export
- Link rewriting for offline browsing
- Concurrent downloads with configurable worker count
- Resource-type filtering (`--download-only`)
- Optional WebP image conversion
- Page limit for testing (`--max-pages`)
- 404 and HTTP error logging with the failing URL

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

## Usage

```bash
# Petrify a website
petrify https://example.com

# Custom output directory and concurrency
petrify https://example.com -o ./my_site -m 4

# Limit pages during testing (does not limit asset downloads)
petrify https://example.com --max-pages 10

# Download only specific resource types
petrify https://example.com --download-only html,css,js,images
```

### Options

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--depth` | `-d` | Maximum crawling depth | unlimited |
| `--max-pages` | | Max pages to scan/process (0 = unlimited) | 0 |
| `--download-only` | | Resource types to download | js,css,images,video,html,pdf |
| `--max-concurrent` | `-m` | Concurrent workers | CPU cores |
| `--output` | `-o` | Output directory | ./petrified_site |
| `--download-external` | | Download external resources | true |
| `--convert-to-webp` | | Convert images to WebP | true |
| `--webp-quality` | | WebP quality (1–100) | 75 |
| `--webp-lossless` | | Lossless WebP | false |
| `--ignore-robots` | `-i` | Ignore robots.txt | true |
| `--timeout` | | Request timeout (seconds) | 270 |

### Resource types (`--download-only`)

`html`, `css`, `js`, `images`, `video`, `pdf`, `fonts`, `other`

> **Note:** `--max-pages` limits page discovery and processing only. Assets referenced by those pages are still queued and downloaded.

### Logging

Set `RUST_LOG` to see download details and 404s:

```bash
RUST_LOG=petrify=warn petrify https://example.com
```

## Output layout

```
petrified_site/
├── index.html
├── about.html
└── static/
    ├── css/
    ├── js/
    ├── images/
    ├── video/
    ├── pdf/
    ├── fonts/
    └── other/
```

## Development

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --all
```

Integration tests use the fixture in `test_site/`. A manual smoke test:

```bash
./test_petrify.sh
```

## Limitations

- Static content only (no JavaScript rendering)
- `--depth` and `--download-external` are parsed but not fully enforced yet
- `robots.txt` is not consulted despite `--ignore-robots`
- Large sites can take a long time and significant disk space

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE).
