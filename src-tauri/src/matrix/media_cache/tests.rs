//! Unit tests for P7.3 media cache index.

use super::*;

fn entry(handle: &str, size: u64, ts: u64) -> CacheEntry {
    CacheEntry {
        handle_id: handle.into(),
        mxc_uri: Some(format!("mxc://example.org/{handle}")),
        size_bytes: size,
        last_access_ts: ts,
        mime_type: Some("image/jpeg".into()),
    }
}

#[test]
fn marker_stable() {
    assert_eq!(matrix_media_cache_markers(), MATRIX_MEDIA_CACHE_MARKER);
}

#[test]
fn upsert_touch_total() {
    let mut idx = MediaCacheIndex::new(1);
    idx.upsert(entry("a", 100, 10)).unwrap();
    idx.upsert(entry("b", 50, 20)).unwrap();
    assert_eq!(idx.len(), 2);
    assert_eq!(idx.total_bytes(), 150);
    idx.touch("a", 30).unwrap();
    assert_eq!(idx.get("a").unwrap().last_access_ts, 30);
    // Replace size
    idx.upsert(entry("a", 80, 30)).unwrap();
    assert_eq!(idx.total_bytes(), 130);
}

#[test]
fn lru_eviction_plan() {
    let mut idx = MediaCacheIndex::new(2);
    idx.upsert(entry("old", 100, 1)).unwrap();
    idx.upsert(entry("mid", 100, 2)).unwrap();
    idx.upsert(entry("new", 100, 3)).unwrap();
    // Budget 150 bytes → drop oldest until under
    let plan = idx.plan_eviction(150, 10);
    assert_eq!(plan, vec!["old".to_string(), "mid".to_string()]);
    assert_eq!(idx.apply_eviction(&plan), 2);
    assert_eq!(idx.len(), 1);
    assert_eq!(idx.total_bytes(), 100);
    assert!(idx.get("new").is_some());
}

#[test]
fn purge_older_and_retire() {
    let mut idx = MediaCacheIndex::new(3);
    idx.upsert(entry("stale", 10, 5)).unwrap();
    idx.upsert(entry("fresh", 10, 100)).unwrap();
    assert_eq!(idx.purge_older_than(50), 1);
    assert!(idx.get("stale").is_none());
    assert!(idx.get("fresh").is_some());
    idx.retire_generation(9);
    assert!(idx.is_empty());
    assert_eq!(idx.session_generation(), 9);
}

#[test]
fn forbids_data_and_tokens() {
    let mut idx = MediaCacheIndex::new(1);
    let err = idx
        .upsert(CacheEntry {
            handle_id: "data:image/png;base64,AAA".into(),
            mxc_uri: None,
            size_bytes: 1,
            last_access_ts: 1,
            mime_type: None,
        })
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p7.3-forbidden-scheme");

    let err = idx
        .upsert(CacheEntry {
            handle_id: "ok".into(),
            mxc_uri: Some("mxc://x/y?access_token=secret".into()),
            size_bytes: 1,
            last_access_ts: 1,
            mime_type: None,
        })
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p7.3-forbidden-id");
}

#[test]
fn list_lru_order() {
    let mut idx = MediaCacheIndex::new(1);
    idx.upsert(entry("c", 1, 30)).unwrap();
    idx.upsert(entry("a", 1, 10)).unwrap();
    idx.upsert(entry("b", 1, 20)).unwrap();
    let ids: Vec<_> = idx
        .list_lru()
        .into_iter()
        .map(|e| e.handle_id.as_str())
        .collect();
    assert_eq!(ids, vec!["a", "b", "c"]);
}
