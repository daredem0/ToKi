use std::borrow::Borrow;
use std::collections::BTreeMap;

#[derive(Debug)]
struct CacheEntry<V> {
    value: V,
    last_used_frame: u64,
}

/// A small LRU-style cache for render resources that are reused across frames.
///
/// Frame lifecycle contract:
/// - call `begin_frame()` when starting a new submission cycle
/// - use `get_or_try_insert_with()` / `mark_used()` while preparing work for that frame
/// - once the previous frame's per-frame instance data has been cleared, call
///   `evict_unused_lru()` to drop least-recently-used entries that were not touched in the
///   current frame
///
/// This cache intentionally does not memoize creation failures. A failed insertion attempt leaves
/// the cache unchanged so a later frame can retry the same key.
#[derive(Debug)]
pub(crate) struct PerFrameLruCache<K, V> {
    entries: BTreeMap<K, CacheEntry<V>>,
    frame_counter: u64,
    capacity: usize,
}

impl<K, V> PerFrameLruCache<K, V>
where
    K: Ord + Clone,
{
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            entries: BTreeMap::new(),
            frame_counter: 0,
            capacity,
        }
    }

    pub(crate) fn begin_frame(&mut self) {
        self.frame_counter = self.frame_counter.saturating_add(1);
    }

    pub(crate) fn clear(&mut self) {
        self.entries.clear();
    }

    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.entries.get(key).map(|entry| &entry.value)
    }

    pub(crate) fn mark_used<Q>(&mut self, key: &Q)
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.last_used_frame = self.frame_counter;
        }
    }

    pub(crate) fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.entries.values_mut().map(|entry| &mut entry.value)
    }

    pub(crate) fn get_or_try_insert_with<E, F>(
        &mut self,
        key: K,
        create: F,
    ) -> Result<Option<&mut V>, E>
    where
        F: FnOnce() -> Result<V, E>,
    {
        if self.entries.contains_key(&key) {
            self.mark_used(&key);
            return Ok(self.entries.get_mut(&key).map(|entry| &mut entry.value));
        }

        let value = create()?;
        self.entries.insert(
            key.clone(),
            CacheEntry {
                value,
                last_used_frame: self.frame_counter,
            },
        );
        Ok(self.entries.get_mut(&key).map(|entry| &mut entry.value))
    }

    pub(crate) fn evict_unused_lru(&mut self) {
        while self.entries.len() > self.capacity {
            let Some(eviction_key) = self
                .entries
                .iter()
                .filter(|(_, entry)| entry.last_used_frame < self.frame_counter)
                .min_by_key(|(_, entry)| entry.last_used_frame)
                .map(|(key, _)| key.clone())
            else {
                break;
            };
            self.entries.remove(&eviction_key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PerFrameLruCache;

    #[test]
    fn reuses_existing_entry_for_same_key() {
        let mut cache = PerFrameLruCache::<String, usize>::new(4);
        cache.begin_frame();
        let first = cache
            .get_or_try_insert_with("hero".to_string(), || Ok::<_, ()>(1))
            .expect("first insert should succeed")
            .expect("entry should be returned");
        *first = 7;

        let second = cache
            .get_or_try_insert_with("hero".to_string(), || Ok::<_, ()>(99))
            .expect("cache hit should succeed")
            .expect("entry should be returned");
        assert_eq!(*second, 7);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn evicts_unused_lru_entries_after_frame_advance() {
        let mut cache = PerFrameLruCache::<String, usize>::new(2);

        cache.begin_frame();
        cache
            .get_or_try_insert_with("a".to_string(), || Ok::<_, ()>(1))
            .expect("insert a")
            .expect("a entry");
        cache
            .get_or_try_insert_with("b".to_string(), || Ok::<_, ()>(2))
            .expect("insert b")
            .expect("b entry");

        cache.begin_frame();
        cache
            .get_or_try_insert_with("b".to_string(), || Ok::<_, ()>(20))
            .expect("reuse b")
            .expect("b entry");
        cache
            .get_or_try_insert_with("c".to_string(), || Ok::<_, ()>(3))
            .expect("insert c")
            .expect("c entry");
        cache.evict_unused_lru();

        assert!(cache.get(&"a".to_string()).is_none());
        assert!(cache.get(&"b".to_string()).is_some());
        assert!(cache.get(&"c".to_string()).is_some());
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn does_not_evict_entries_used_in_current_frame() {
        let mut cache = PerFrameLruCache::<String, usize>::new(1);

        cache.begin_frame();
        cache
            .get_or_try_insert_with("a".to_string(), || Ok::<_, ()>(1))
            .expect("insert a")
            .expect("a entry");
        cache
            .get_or_try_insert_with("b".to_string(), || Ok::<_, ()>(2))
            .expect("insert b")
            .expect("b entry");
        cache.evict_unused_lru();

        assert_eq!(cache.len(), 2);
        assert!(cache.get(&"a".to_string()).is_some());
        assert!(cache.get(&"b".to_string()).is_some());
    }

    #[test]
    fn clear_removes_all_entries() {
        let mut cache = PerFrameLruCache::<String, usize>::new(4);
        cache.begin_frame();
        cache
            .get_or_try_insert_with("hero".to_string(), || Ok::<_, ()>(1))
            .expect("insert should succeed");
        cache.clear();
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn failed_creation_is_not_cached_and_can_be_retried() {
        let mut cache = PerFrameLruCache::<String, usize>::new(4);
        cache.begin_frame();

        let failed = cache.get_or_try_insert_with("hero".to_string(), || Err::<usize, _>("nope"));
        assert!(failed.is_err());
        assert_eq!(cache.len(), 0);

        let succeeded = cache
            .get_or_try_insert_with("hero".to_string(), || Ok::<_, &str>(5))
            .expect("retry should succeed")
            .expect("entry should be returned");
        assert_eq!(*succeeded, 5);
        assert_eq!(cache.len(), 1);
    }
}
