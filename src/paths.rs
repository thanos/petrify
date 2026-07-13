use std::path::Path;

/// Normalize an output directory path (trim trailing slashes).
pub fn normalize_output_dir(output_dir: &str) -> String {
    output_dir.trim_end_matches('/').replace('\\', "/")
}

/// Convert a local filesystem path under `output_dir` to a site-root-relative
/// path (e.g. `/static/images/logo.webp`) for hosting behind a web server.
pub fn site_root_path(output_dir: &str, local_path: &str) -> String {
    let output_dir = normalize_output_dir(output_dir);
    let local_path = local_path.replace('\\', "/");

    if let Some(relative) = local_path.strip_prefix(&output_dir) {
        let path = relative.trim_start_matches('/');
        return format!("/{path}");
    }

    if let Some(pos) = local_path.find("/static/") {
        return local_path[pos..].to_string();
    }

    format!("/{}", local_path.trim_start_matches('/'))
}

pub fn update_path_to_webp(original_path: &str) -> String {
    let normalized = original_path.replace('\\', "/");
    let path = Path::new(&normalized);
    if let Some(stem) = path.file_stem() {
        let webp_filename = format!("{}.webp", stem.to_string_lossy());
        if let Some(parent) = path.parent() {
            let parent = parent.to_string_lossy();
            if parent.is_empty() || parent == "." {
                webp_filename
            } else {
                format!("{}/{}", parent, webp_filename)
            }
        } else {
            webp_filename
        }
    } else {
        normalized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn site_root_path_handles_trailing_slash_in_output_dir() {
        assert_eq!(
            site_root_path("./mb/", "./mb//static/images/foo.webp"),
            "/static/images/foo.webp"
        );
        assert_eq!(
            site_root_path("./mb/", "./mb/static/images/foo.webp"),
            "/static/images/foo.webp"
        );
    }

    #[test]
    fn normalize_output_dir_trims_trailing_slash() {
        assert_eq!(normalize_output_dir("./mb/"), "./mb");
        assert_eq!(normalize_output_dir("/output/"), "/output");
    }

    #[test]
    fn site_root_path_strips_output_prefix() {
        assert_eq!(
            site_root_path("/output", "/output/static/css/app.css"),
            "/static/css/app.css"
        );
        assert_eq!(
            site_root_path("./mb", "./mb/artfestival/artist/raed-issa/index.html"),
            "/artfestival/artist/raed-issa/index.html"
        );
    }

    #[test]
    fn update_path_to_webp_changes_extension() {
        assert_eq!(
            update_path_to_webp("/out/static/images/photo.png"),
            "/out/static/images/photo.webp"
        );
        assert_eq!(
            update_path_to_webp("/out/static/images/photo.JPG"),
            "/out/static/images/photo.webp"
        );
    }
}
