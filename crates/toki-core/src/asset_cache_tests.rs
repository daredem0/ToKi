use super::AssetCache;
use std::cell::RefCell;

#[test]
fn new_cache_is_empty() {
    let cache: AssetCache<String, i32> = AssetCache::new();
    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
}

#[test]
fn get_returns_none_for_missing_key() {
    let cache: AssetCache<String, i32> = AssetCache::new();
    assert!(cache.get(&"missing".to_string()).is_none());
}

#[test]
fn insert_adds_item_to_cache() {
    let mut cache: AssetCache<String, i32> = AssetCache::new();
    cache.insert("key".to_string(), 42);

    assert_eq!(cache.len(), 1);
    assert!(!cache.is_empty());
    assert_eq!(cache.get(&"key".to_string()), Some(&42));
}

#[test]
fn get_or_load_returns_cached_value_without_calling_loader() {
    let mut cache: AssetCache<String, i32> = AssetCache::new();
    cache.insert("key".to_string(), 42);

    let loader_called = RefCell::new(false);
    let result = cache.get_or_load("key".to_string(), |_| {
        *loader_called.borrow_mut() = true;
        Ok::<i32, String>(99)
    });

    assert_eq!(result, Ok(42));
    assert!(!*loader_called.borrow());
}

#[test]
fn get_or_load_calls_loader_and_caches_result_on_miss() {
    let mut cache: AssetCache<String, i32> = AssetCache::new();

    let loader_called = RefCell::new(false);
    let result = cache.get_or_load("key".to_string(), |key| {
        *loader_called.borrow_mut() = true;
        assert_eq!(key, "key");
        Ok::<i32, String>(42)
    });

    assert_eq!(result, Ok(42));
    assert!(*loader_called.borrow());
    assert_eq!(cache.get(&"key".to_string()), Some(&42));
}

#[test]
fn get_or_load_propagates_loader_error() {
    let mut cache: AssetCache<String, i32> = AssetCache::new();

    let result = cache.get_or_load("key".to_string(), |_| {
        Err::<i32, String>("load failed".to_string())
    });

    assert_eq!(result, Err("load failed".to_string()));
    assert!(cache.is_empty());
}

#[test]
fn get_or_load_does_not_cache_on_error() {
    let mut cache: AssetCache<String, i32> = AssetCache::new();

    let _ = cache.get_or_load("key".to_string(), |_| {
        Err::<i32, String>("load failed".to_string())
    });

    assert!(cache.get(&"key".to_string()).is_none());
}

#[test]
fn clear_removes_all_items() {
    let mut cache: AssetCache<String, i32> = AssetCache::new();
    cache.insert("a".to_string(), 1);
    cache.insert("b".to_string(), 2);

    assert_eq!(cache.len(), 2);
    cache.clear();
    assert!(cache.is_empty());
}

#[test]
fn works_with_pathbuf_keys() {
    use std::path::PathBuf;

    let mut cache: AssetCache<PathBuf, String> = AssetCache::new();
    let path = PathBuf::from("/some/path/file.json");

    let result = cache.get_or_load(path.clone(), |p| {
        Ok::<String, String>(format!("loaded from {}", p.display()))
    });

    assert_eq!(result, Ok("loaded from /some/path/file.json".to_string()));
    assert!(cache.get(&path).is_some());
}

#[test]
fn second_get_or_load_uses_cache() {
    let mut cache: AssetCache<String, i32> = AssetCache::new();
    let load_count = RefCell::new(0);

    let loader = |_: &String| {
        *load_count.borrow_mut() += 1;
        Ok::<i32, String>(42)
    };

    let _ = cache.get_or_load("key".to_string(), loader);
    let _ = cache.get_or_load("key".to_string(), loader);

    assert_eq!(*load_count.borrow(), 1);
}
