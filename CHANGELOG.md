# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-07-13

### Added

- Fonts (`.woff`, `.woff2`, `.ttf`, `.otf`, `.eot`) are included in the default `--download-only` list.
- SEO / social metadata image discovery and rewriting:
  - `<meta property="og:image…">`, `twitter:image`, and related `content` URLs
  - JSON-LD fields such as `image`, `thumbnailUrl`, `contentUrl`, `logo`, and `photo`
- Discovery of CSS `url(...)` values inside HTML `style` attributes (for example UIkit cover backgrounds).
- Site-root URL rewriting for hosted mirrors: assets and pages rewrite to paths like `/static/images/logo.webp` and `/about/index.html` so the output works behind a normal web server document root.
- Shared `paths` helpers for output-dir normalization, site-root paths, and WebP filename updates.
- `--deep` to crawl the whole same-host site with unlimited depth.
- `--stay-on-site` (alias `--stay-on-same-domain`) to skip third-party / CDN assets.
- Crawl `--depth` is now enforced (`0` = starting page only).
- Off-site HTML pages are never mirrored or rewritten; off-site assets (images, documents, CSS, JS, fonts, video/audio, other) remain downloadable when external downloads are enabled.
- Document extensions beyond PDF (`.doc`, `.docx`, `.epub`, spreadsheets, etc.) are classified under the `pdf` download filter.

### Fixed

- Empty `src=""` attributes no longer corrupt HTML (`String::replace("", …)` inserted paths between every character).
- `href="/"` no longer globally replaces every `/` in the document (which broke tags such as `</body>`).
- Trailing slash on `--output` (for example `-o ./mb/`) no longer produces protocol-relative `//static/...` URLs that browsers resolve as host `static`.
- URL fragments (`#home`, `#program`) are preserved on rewrite instead of becoming `%23….html` paths.
- When WebP conversion is enabled, rewritten HTML links use `.webp` paths that match the files written to disk.
- Absolute path resolution uses `Url::join` so fragments and queries are not treated as path segments.
- `--download-external=false` / `--stay-on-site` now hard-gates off-host asset downloads.

### Changed

- Rewritten links are site-root absolute (`/…`) rather than page-relative (`../…`), which is required when serving the mirror from a web server.
- Default `--download-only` is now `js,css,images,video,html,pdf,fonts`.

## [0.1.0] - 2025-06-09

### Added

- Initial public release of `petrify` as a CLI and library.
- Concurrent crawl and download of same-host HTML pages.
- Asset download for CSS, JavaScript, images, video, and PDF.
- Optional JPEG/PNG/GIF → WebP conversion.
- HTML link rewriting for common URL attributes and inline CSS `@import` / `url(...)`.
- Progress reporting and `RUST_LOG` logging.

[0.2.0]: https://github.com/thanos/petrify/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/thanos/petrify/releases/tag/v0.1.0
