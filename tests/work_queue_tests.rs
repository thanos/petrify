use petrify::types::WorkQueue;
use url::Url;

#[test]
fn deduplicates_pages_and_resources() {
    let mut queue = WorkQueue::new();
    let page = Url::parse("https://example.com/about").unwrap();
    let resource = Url::parse("https://example.com/app.js").unwrap();

    queue.add_page(page.clone());
    queue.add_page(page);
    queue.add_resource(resource.clone());
    queue.add_resource(resource);

    assert_eq!(queue.pages.len(), 1);
    assert_eq!(queue.resources.len(), 1);
}

#[test]
fn normalizes_urls_without_query_or_fragment() {
    let queue = WorkQueue::new();
    let with_query = Url::parse("https://example.com/page?q=1#section").unwrap();
    let without = Url::parse("https://example.com/page").unwrap();

    assert_eq!(
        queue.normalize_url(&with_query),
        queue.normalize_url(&without)
    );
}
