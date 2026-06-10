use std::str;
use std::time::{Duration, Instant};

use bytes::Bytes;
use dashmap::DashMap;

#[derive(Debug, Clone)]
pub struct Entry {
    pub value: Bytes,
    pub expires_at: Option<Instant>,
}

impl Entry {
    pub fn is_expired(&self) -> bool {
        self.expires_at.map(|t| Instant::now() > t).unwrap_or(false)
    }
}

#[derive(Debug, Clone)]
pub struct Store {
    map: DashMap<Bytes, Entry>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            map: DashMap::new(),
        }
    }

    pub fn get(&self, key: &Bytes) -> Option<Bytes> {
        let entry = self.map.get(key)?;
        if entry.is_expired() {
            drop(entry);
            self.map.remove(key);
            return None;
        } else {
            Some(entry.value.clone())
        }
    }

    pub fn set(&self, key: Bytes, value: Bytes, ex: Option<u64>) {
        let expires_at = ex.map(|secs| Instant::now() + Duration::from_secs(secs));
        self.map.insert(key, Entry { value, expires_at });
    }

    pub fn del(&self, keys: &[Bytes]) -> i64 {
        keys.iter()
            .filter(|key| self.map.remove(*key).is_some())
            .count() as i64
    }

    pub fn exists(&self, keys: &[Bytes]) -> i64 {
        keys.iter()
            .filter(|key| {
                self.map
                    .get(*key)
                    .map(|entry| !entry.is_expired())
                    .unwrap_or(false)
            })
            .count() as i64
    }

    pub fn incr(&self, key: Bytes) -> Result<i64, &'static str> {
        let mut result = Ok(1_i64);
        self.map
            .entry(key)
            .and_modify(|entry| {
                let current = str::from_utf8(&entry.value)
                    .ok()
                    .and_then(|s| s.parse::<i64>().ok());
                match current {
                    Some(val) => {
                        let new = val + 1;
                        let mut buf = itoa::Buffer::new();
                        entry.value = Bytes::copy_from_slice(buf.format(new).as_bytes());
                        result = Ok(new);
                    }
                    None => result = Err("ERR value is not an integer or out of range"),
                }
            })
            .or_insert_with(|| Entry {
                value: Bytes::from_static(b"1"),
                expires_at: None,
            });
        result
    }

    pub fn expire(&self, key: &Bytes, secs: u64) -> i64 {
        let mut entry = match self.map.get_mut(key) {
            Some(entry) => entry,
            None => return 0,
        };

        entry.expires_at = Some(Instant::now() + Duration::from_secs(secs));
        1
    }

    pub fn ttl(&self, key: &Bytes) -> i64 {
        let entry = match self.map.get(key) {
            Some(entry) => entry,
            None => return -2,
        };

        let expires_at = match entry.expires_at {
            Some(exp) => exp,
            None => return -1,
        };

        let now = Instant::now();
        if now >= expires_at {
            drop(entry);
            self.map.remove(key);
            return -2;
        }

        expires_at.duration_since(now).as_secs() as i64
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use bytes::Bytes;
    use tokio::task;

    use crate::store::Store;

    #[test]
    fn get_missing_key() {
        let store = Store::new();

        assert_eq!(store.get(&Bytes::from("foo")), None,);
    }

    #[test]
    fn set_and_get() {
        let store = Store::new();

        store.set(Bytes::from("foo"), Bytes::from("bar"), None);

        assert_eq!(store.get(&Bytes::from("foo")), Some(Bytes::from("bar")));
    }

    #[test]
    fn del_existing_keys() {
        let store = Store::new();

        store.set(Bytes::from("a"), Bytes::from("1"), None);

        store.set(Bytes::from("b"), Bytes::from("2"), None);

        let deleted = store.del(&[Bytes::from("a"), Bytes::from("c")]);

        assert_eq!(deleted, 1);

        assert_eq!(store.get(&Bytes::from("a")), None,);

        assert_eq!(store.get(&Bytes::from("b")), Some(Bytes::from("2")),);
    }

    #[test]
    fn exists_counts_duplicates() {
        let store = Store::new();

        store.set(Bytes::from("a"), Bytes::from("1"), None);

        let count = store.exists(&[Bytes::from("a"), Bytes::from("a"), Bytes::from("a")]);

        assert_eq!(count, 3);
    }

    #[test]
    fn ttl_missing_key() {
        let store = Store::new();

        assert_eq!(store.ttl(&Bytes::from("missing")), -2);
    }

    #[test]
    fn ttl_without_expiry() {
        let store = Store::new();

        store.set(Bytes::from("k"), Bytes::from("v"), None);

        assert_eq!(store.ttl(&Bytes::from("k")), -1);
    }

    #[test]
    fn expire_existing_key() {
        let store = Store::new();

        store.set(Bytes::from("k"), Bytes::from("v"), None);

        assert_eq!(store.expire(&Bytes::from("k"), 60,), 1);
    }

    #[test]
    fn expire_missing_key() {
        let store = Store::new();

        assert_eq!(store.expire(&Bytes::from("missing"), 60,), 0);
    }

    #[tokio::test]
    async fn ttl_expiry() {
        let store = Store::new();

        store.set(Bytes::from("key"), Bytes::from("value"), Some(1));

        tokio::time::sleep(Duration::from_millis(1100)).await;

        assert_eq!(store.get(&Bytes::from("key"),), None,);
    }

    #[test]
    fn incr_non_integer() {
        let store = Store::new();

        store.set(Bytes::from("counter"), Bytes::from("hello"), None);

        assert!(store.incr(Bytes::from("counter")).is_err());
    }

    #[test]
    fn incr_missing_key() {
        let store = Store::new();

        assert_eq!(store.incr(Bytes::from("counter")).unwrap(), 1,);
    }

    #[test]
    fn incr_three_times() {
        let store = Store::new();

        assert_eq!(store.incr(Bytes::from("counter")).unwrap(), 1,);

        assert_eq!(store.incr(Bytes::from("counter")).unwrap(), 2,);

        assert_eq!(store.incr(Bytes::from("counter")).unwrap(), 3,);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn concurrent_store_test() {
        let store = Arc::new(Store::new());

        let mut handles = Vec::new();

        for thread_id in 0..10 {
            let store = Arc::clone(&store);

            handles.push(task::spawn(async move {
                for i in 0..1000 {
                    let key = Bytes::from(format!("k{thread_id}:{i}"));

                    let value = Bytes::from(format!("v{thread_id}:{i}"));

                    store.set(key.clone(), value.clone(), None);

                    let fetched = store.get(&key);

                    assert_eq!(fetched, Some(value),);
                }
            }));
        }

        for handle in handles {
            handle.await.unwrap();
        }
    }
}
