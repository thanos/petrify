use petrify::paths::{normalize_output_dir, site_root_path, update_path_to_webp};

#[test]
fn normalize_output_dir_strips_trailing_slashes() {
    assert_eq!(normalize_output_dir("./mb/"), "./mb");
    assert_eq!(normalize_output_dir("./mb///"), "./mb");
    assert_eq!(normalize_output_dir("/abs/out/"), "/abs/out");
}

#[test]
fn site_root_path_never_emits_protocol_relative_double_slash() {
    // Regression: `-o ./mb/` produced `./mb//static/...` → `//static/...`
    // which browsers treat as host `static`.
    assert_eq!(
        site_root_path("./mb/", "./mb//static/images/foo.webp"),
        "/static/images/foo.webp"
    );
    assert_eq!(
        site_root_path("./mb/", "./mb/static/css/app.css"),
        "/static/css/app.css"
    );
    assert_eq!(
        site_root_path("./mb/", "./mb/artfestival/artist/x/index.html"),
        "/artfestival/artist/x/index.html"
    );
}

#[test]
fn site_root_path_maps_index_and_assets() {
    assert_eq!(
        site_root_path("/output", "/output/index.html"),
        "/index.html"
    );
    assert_eq!(
        site_root_path("/output", "/output/static/fonts/brand.woff2"),
        "/static/fonts/brand.woff2"
    );
}

#[test]
fn update_path_to_webp_handles_mixed_case_extensions() {
    assert_eq!(
        update_path_to_webp("./mb/static/images/photo.JPG"),
        "./mb/static/images/photo.webp"
    );
    assert_eq!(
        update_path_to_webp("./mb/static/images/photo.jpeg"),
        "./mb/static/images/photo.webp"
    );
}
