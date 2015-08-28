use std::collections::HashMap;
use std::hash::Hash;
use std::borrow::Borrow;
use std::io::{ Read, Write };

#[derive(Copy, Clone)]
pub enum OutOfMemoryStrategy {
    Fail,
    Restart,
}

use history::History;
use StoreResult;
use Cache;
use CreateReaderError;
use CreateWriterError;

/// In-memory cache.
pub struct MemCache<K: Clone> {
    limit: u64,
    history: History<K>,
    items: HashMap<K, Vec<u8>>,
}

impl<K: Clone> MemCache<K>
    where
        K: Eq + Hash
{

    pub fn new(limit: u64) -> MemCache<K> {
        let mut bucker_size = limit / 5;
        if bucker_size == 0 {
            bucker_size = 1;
        }
        let bucket_count = 2;

        MemCache::<K> {
            limit: limit,
            history: History::new(bucker_size, bucket_count),
            items: HashMap::new(),
        }
    }

    pub fn with_capacity(limit: u64) -> MemCache<K> {
        Self::new(limit)
    }

    pub fn limit(&self) -> u64 {
        self.limit
    }

    pub fn usage(&self) -> u64 {
        self.history.usage()
    }

    pub fn detailed_usage(&self) -> Vec<(u64, Option<u64>)> {
        self.history.detailed_usage()
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.history.clear();
    }

    pub fn can_store_bytes(&self, amount: u64) -> bool {
        self.usage() + amount <= self.limit
    }

    fn free_memory(&mut self, required_mem: u64) -> bool {
        if self.can_store_bytes(required_mem) {
            return true;
        }

        let mut spilled = Vec::new();
        loop {
            self.history.spill(&mut spilled);
            for &(ref key, _) in spilled.iter() {
                self.items.remove(&key);
            }
            spilled.clear();

            if spilled.len() == 0 {
                if !self.can_store_bytes(required_mem) {
                    return false;
                } else {
                    break;
                }
            }

            if self.can_store_bytes(required_mem) {
                break;
            }
        }

        true
    }

    pub fn set(&mut self, key: K, value: Vec<u8>) -> StoreResult {
        let new_required_mem = value.len() as u64;
        let existing_item_memory_use = match self.items.get(&key) {
            Some(ref v) => Some(v.len() as u64),
            None => None,
        };

        let real_required_mem = match existing_item_memory_use {
            Some(existing) => if existing >= new_required_mem { 0 } else { new_required_mem - existing },
            None => new_required_mem,
        };

        if !self.free_memory(real_required_mem) {
            if let Some(_) = self.items.remove(&key) {
                self.history.remove(&key);
            }
            return StoreResult::OutOfMemory;
        }

        self.items.insert(key.clone(), value);
        self.history.hit(key, new_required_mem);

        StoreResult::Stored
    }

    /// Get cached value.
    pub fn get<A: Borrow<K>>(&mut self, key: A) -> Option<&[u8]> {
        let res = self.items.get(key.borrow());

        if let Some(ref res) = res {
            self.history.hit(key.borrow().clone(), res.len() as u64);
        }

        res.map(|v| v.borrow())
    }
}

impl<K: Clone> Cache<K> for MemCache<K> {
    fn fetch<R: Read>(&self, key: K) -> Result<R, CreateReaderError> {
        Err(CreateReaderError::NotFound)
    }

    fn store<W: Write>(&self, key: K, required_mem: u64) -> Result<W, CreateWriterError> {
        Err(CreateWriterError::OutOfMemory)
    }
}

#[cfg(test)]
mod test {
    use StoreResult;
    use super::*;

    #[test]
    fn store_and_get() {
        let mut cache = MemCache::with_capacity(1000);
        cache.set("test", vec![2, 3, 4]);
        assert_eq!(&[2, 3, 4], cache.get("test").unwrap());
    }

    #[test]
    fn should_not_get_not_stored() {
        let mut cache = MemCache::<u8>::with_capacity(1000);
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
        assert_eq!(&[2, 3, 4], cache.get("test").unwrap());
    }

    #[test]
    fn prefer_not_storing_new_value_if_it_is_quite_big() {
        let mut cache = MemCache::with_capacity(3);
        assert_eq!(StoreResult::Stored, cache.set("test", vec![2, 3]));
        assert_eq!(StoreResult::OutOfMemory, cache.set("test2", vec![3, 4, 5]));
        assert_eq!(&[2, 3], cache.get("test").unwrap());
        assert_eq!(None, cache.get("test2"));
    }

    #[test]
    fn should_keep_old_if_new_does_not_fit() {
        let mut cache = MemCache::with_capacity(2);
        cache.set("test", vec![2, 3]);
        cache.set("test2", vec![3, 4, 5]);
        assert_eq!(None, cache.get("test2"));
        assert_eq!(&[2, 3], cache.get("test").unwrap());
    }
}
