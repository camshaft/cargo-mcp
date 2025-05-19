use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::sync::Mutex;

//= docs/design/technical-spec.md#caching-requirements
//# The server MUST implement a caching system for documentation and crate information.

//= docs/design/technical-spec.md#caching-requirements
//# The server MUST store cache entries with timestamps.
pub struct Cache<T> {
    entries: Arc<Mutex<HashMap<String, CacheEntry<T>>>>,
    ttl: Duration,
    max_size: usize,
}

struct CacheEntry<T> {
    data: T,
    timestamp: SystemTime,
}

impl<T: Clone> Cache<T> {
    pub fn new(ttl: Duration, max_size: usize) -> Self {
        Self {
            entries: Arc::new(Mutex::new(HashMap::new())),
            ttl,
            max_size,
        }
    }

    //= docs/design/technical-spec.md#caching-requirements
    //# The server MUST validate cache entries before returning them.
    pub async fn get(&self, key: &str) -> Option<T> {
        let cache = self.entries.lock().await;
        if let Some(entry) = cache.get(key) {
            if self.is_valid(entry) {
                return Some(entry.data.clone());
            }
        }
        None
    }

    pub async fn insert(&self, key: String, value: T) {
        let mut cache = self.entries.lock().await;

        //= docs/design/technical-spec.md#caching-requirements
        //# The server SHOULD implement a least-recently-used (LRU) eviction policy when the cache is full.
        if cache.len() >= self.max_size {
            // Simple LRU: remove oldest entry
            if let Some(oldest_key) = cache
                .iter()
                .min_by_key(|(_, entry)| entry.timestamp)
                .map(|(k, _)| k.clone())
            {
                cache.remove(&oldest_key);
            }
        }

        cache.insert(
            key,
            CacheEntry {
                data: value,
                timestamp: SystemTime::now(),
            },
        );
    }

    //= docs/design/technical-spec.md#caching-requirements
    //# The server MUST implement time-to-live (TTL) for cache entries.
    fn is_valid(&self, entry: &CacheEntry<T>) -> bool {
        SystemTime::now()
            .duration_since(entry.timestamp)
            .map(|age| age < self.ttl)
            .unwrap_or(false)
    }

    //= docs/design/technical-spec.md#caching-requirements
    //# The server MUST remove invalid cache entries when detected.
    pub async fn cleanup(&self) {
        let mut cache = self.entries.lock().await;
        cache.retain(|_, entry| self.is_valid(entry));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{Duration, sleep};

    #[tokio::test]
    async fn test_cache_ttl() {
        let cache = Cache::new(Duration::from_millis(100), 10);
        cache.insert("key".to_string(), "value".to_string()).await;

        // Value should be available immediately
        assert_eq!(cache.get("key").await, Some("value".to_string()));

        // Wait for TTL to expire
        sleep(Duration::from_millis(150)).await;

        // Value should be gone
        assert_eq!(cache.get("key").await, None);
    }

    #[tokio::test]
    async fn test_cache_max_size() {
        let cache = Cache::new(Duration::from_secs(1), 2);

        // Insert three items (exceeding max size)
        cache.insert("key1".to_string(), "value1".to_string()).await;
        sleep(Duration::from_millis(10)).await; // Ensure different timestamps
        cache.insert("key2".to_string(), "value2".to_string()).await;
        sleep(Duration::from_millis(10)).await;
        cache.insert("key3".to_string(), "value3".to_string()).await;

        // Oldest entry (key1) should be gone
        assert_eq!(cache.get("key1").await, None);
        assert_eq!(cache.get("key2").await, Some("value2".to_string()));
        assert_eq!(cache.get("key3").await, Some("value3".to_string()));
    }

    #[tokio::test]
    async fn test_cache_cleanup() {
        let cache = Cache::new(Duration::from_millis(100), 10);
        cache.insert("key1".to_string(), "value1".to_string()).await;
        sleep(Duration::from_millis(150)).await;
        cache.insert("key2".to_string(), "value2".to_string()).await;

        cache.cleanup().await;

        // key1 should be removed, key2 should remain
        assert_eq!(cache.get("key1").await, None);
        assert_eq!(cache.get("key2").await, Some("value2".to_string()));
    }
}
