use std::collections::HashMap;
use std::hash::Hash;

/// A generic cache for assets that can be loaded from a source.
///
/// This abstracts the common pattern of checking a cache before loading,
/// then inserting the result into the cache.
///
/// # Type Parameters
/// - `K`: The key type used to identify assets (e.g., `PathBuf`, `String`)
/// - `V`: The value type of the cached asset (must be `Clone`)
#[derive(Debug)]
pub struct AssetCache<K, V> {
    cache: HashMap<K, V>,
}

// Manual Default impl to avoid requiring V: Default
impl<K, V> Default for AssetCache<K, V> {
    fn default() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }
}

impl<K, V> AssetCache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    /// Creates a new empty asset cache.
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Returns a reference to the cached value if it exists.
    pub fn get(&self, key: &K) -> Option<&V> {
        self.cache.get(key)
    }

    /// Gets a cached value or loads it using the provided loader function.
    ///
    /// If the key exists in the cache, returns a clone of the cached value.
    /// Otherwise, calls the loader function, inserts the result into the cache,
    /// and returns a clone.
    ///
    /// # Errors
    /// Returns the error from the loader function if loading fails.
    pub fn get_or_load<E, F>(&mut self, key: K, loader: F) -> Result<V, E>
    where
        F: FnOnce(&K) -> Result<V, E>,
    {
        if let Some(cached) = self.cache.get(&key) {
            return Ok(cached.clone());
        }

        let value = loader(&key)?;
        self.cache.insert(key, value.clone());
        Ok(value)
    }

    /// Inserts a value into the cache directly.
    pub fn insert(&mut self, key: K, value: V) {
        self.cache.insert(key, value);
    }

    /// Returns the number of cached items.
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Returns true if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Clears all cached items.
    pub fn clear(&mut self) {
        self.cache.clear();
    }
}

#[cfg(test)]
#[path = "asset_cache_tests.rs"]
mod tests;
