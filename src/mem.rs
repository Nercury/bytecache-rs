use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::hash::Hash;
use std::borrow::Borrow;

#[derive(Copy, Clone)]
pub enum OutOfMemoryStrategy {
    Fail,
    Restart,
}

pub enum StoreResult {
    Stored,
    OutOfMemory,
}

use {
    RequiredBytes,
};

/// Very simple in-memory cache.
pub struct MemCache<K, V> {
    limit: u64,
    usage: u64,
    strategy: OutOfMemoryStrategy,
    items: HashMap<K, V>,
}

impl<K, V> MemCache<K, V>
    where
        K: Eq + Hash,
        V: RequiredBytes
{

    pub fn new(limit: u64, strategy: OutOfMemoryStrategy) -> MemCache<K, V> {
        MemCache::<K, V> {
            limit: limit,
            usage: 0,
            strategy: strategy,
            items: HashMap::new(),
        }
    }

    pub fn with_capacity(limit: u64) -> MemCache<K, V> {
        Self::new(limit, OutOfMemoryStrategy::Restart)
    }

    pub fn limit(&self) -> u64 {
        self.limit
    }

    pub fn usage(&self) -> u64 {
        self.usage
    }

    pub fn strategy(&self) -> OutOfMemoryStrategy {
        self.strategy
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.usage = 0;
    }

    pub fn can_store_bytes(&self, amount: u64) -> bool {
        self.usage + amount <= self.limit
    }

    pub fn set(&mut self, key: K, value: V) -> StoreResult {
        let required_memory = value.required_bytes();

        if !self.can_store_bytes(required_memory) {
            match self.strategy {
                OutOfMemoryStrategy::Fail => return StoreResult::OutOfMemory,
                OutOfMemoryStrategy::Restart => {
                    if required_memory <= self.limit {
                        self.clear();
                    } else {
                        return StoreResult::OutOfMemory;
                    }
                },
            };
        }

        match self.items.entry(key) {
            Entry::Occupied(mut e) => {
                let old = e.insert(value);
                self.usage -= old.required_bytes();
            },
            Entry::Vacant(e) => {
                e.insert(value);
            }
        };

        self.usage += required_memory;

        StoreResult::Stored
    }

    pub fn get<A: Borrow<K>>(&self, key: A) -> Option<&V> {
        self.items.get(key.borrow())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn store_and_get() {
        let mut cache = MemCache::with_capacity(1000);
        cache.set("test", vec![2, 3, 4]);
        assert_eq!(&vec![2, 3, 4], cache.get("test").unwrap());
    }

    #[test]
    fn should_not_get_not_stored() {
        let mut cache = MemCache::<u8, Vec<u8>>::with_capacity(1000);
        assert_eq!(None, cache.get(1));
    }

    #[test]
    fn should_not_store_not_fitting() {
        let mut cache = MemCache::with_capacity(2);
        cache.set("test", vec![2, 3, 4]);
        assert_eq!(None, cache.get("test"));
    }

    #[test]
    fn should_store_exactly_fitting() {
        let mut cache = MemCache::with_capacity(3);
        cache.set("test", vec![2, 3, 4]);
        assert_eq!(&vec![2, 3, 4], cache.get("test").unwrap());
    }

    #[test]
    fn should_replace_old_with_new_fitting() {
        let mut cache = MemCache::with_capacity(3);
        cache.set("test", vec![2, 3]);
        cache.set("test2", vec![3, 4, 5]);
        assert_eq!(&vec![3, 4, 5], cache.get("test2").unwrap());
        assert_eq!(None, cache.get("test"));
    }

    #[test]
    fn should_keep_old_if_new_does_not_fit() {
        let mut cache = MemCache::with_capacity(2);
        cache.set("test", vec![2, 3]);
        cache.set("test2", vec![3, 4, 5]);
        assert_eq!(None, cache.get("test2"));
        assert_eq!(&vec![2, 3], cache.get("test").unwrap());
    }
}
