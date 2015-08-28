use std::collections::VecDeque;
use std::collections::HashMap;
use std::collections::hash_map;
use std::hash::Hash;
use std::borrow::Borrow;
use std::mem;

pub struct Bucket<K> {
    items: HashMap<K, u64>,
    usage: u64,
}

/// Bucket tracks the sum of all inserted values.
///
/// When values are replaced, the sum is correctly adjusted.
impl<K> Bucket<K>
    where K: Hash + Eq
{
    pub fn new() -> Bucket<K> {
        Bucket::<K> {
            items: HashMap::new(),
            usage: 0,
        }
    }

    #[inline(always)]
    pub fn contains<Q: ?Sized>(&self, key: &Q) -> bool
        where
            K: Borrow<Q>,
            Q: Hash + Eq
    {
        self.items.contains_key(key)
    }

    #[inline(always)]
    pub fn get<Q: ?Sized>(&self, key: &Q) -> Option<&u64>
        where
            K: Borrow<Q>,
            Q: Hash + Eq
    {
        self.items.get(key)
    }

    pub fn insert(&mut self, key: K, required_bytes: u64)
    {
        if let Some(old) = self.items.insert(key, required_bytes) {
            self.usage -= old;
        }

        self.usage += required_bytes;
    }

    pub fn remove<Q: ?Sized>(&mut self, key: &Q) -> bool
        where
            K: Borrow<Q>,
            Q: Hash + Eq
    {
        if let Some(value) = self.items.remove(key) {
            self.usage -= value;
            return true;
        }
        false
    }

    pub fn iter(&self) -> hash_map::Iter<K, u64> {
        self.items.iter()
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.usage = 0;
    }

    pub fn usage(&self) -> u64 {
        self.usage
    }
}

impl<K> Extend<(K, u64)> for Bucket<K> where K: Eq + Hash {
    fn extend<T: IntoIterator<Item=(K, u64)>>(&mut self, iter: T) {
        let iter = iter.into_iter();
        if let (_, Some(len)) = iter.size_hint() {
            self.items.reserve(len);
        }
        for (k, v) in iter {
            self.insert(k, v);
        }
    }
}

pub struct History<K> {
    max_bucket_usage: u64,
    bucket_count: u64,
    next_bucket: Bucket<K>,
    old_bucket: Bucket<K>,
    buckets: VecDeque<Bucket<K>>,
}

impl<K> History<K>
    where K: Eq + Hash + Clone
{
    pub fn new(max_bucket_usage: u64, bucket_count: u64) -> History<K> {
        History::<K> {
            max_bucket_usage: max_bucket_usage,
            bucket_count: bucket_count,
            next_bucket: Bucket::new(),
            old_bucket: Bucket::new(),
            buckets: VecDeque::new(),
        }
    }

    /// Refresh the key to delay its removal or to insert it to history.
    pub fn hit(&mut self, key: K, required_bytes: u64)
        where
            K: Eq + Hash
    {
        let next_bucket_usage = self.next_bucket.usage();
        let remaining_space = if self.max_bucket_usage > next_bucket_usage {
            self.max_bucket_usage - next_bucket_usage
        } else {
            0
        };

        if required_bytes > remaining_space {
            self.burry_bucket();
        }

        self.insert(key, required_bytes);
    }

    fn insert(&mut self, key: K, required_bytes: u64)
        where
            K: Eq + Hash
    {
        let maybe_bytes = self.next_bucket.get(&key).map(|v| *v);

        if let Some(bytes) = maybe_bytes {
            if bytes != required_bytes {
                self.next_bucket.insert(key, required_bytes);
            }

            return;
        }

        self.old_bucket.remove(&key);
        self.dig_out(&key);

        self.next_bucket.insert(key, required_bytes);
    }

    /// Remove key from history.
    pub fn remove<Q: ?Sized>(&mut self, key: &Q) -> bool
        where
            K: Borrow<Q>,
            Q: Eq + Hash
    {
        if self.next_bucket.remove(key) {
            return true;
        }

        if self.dig_out(key) {
            return true;
        }

        self.old_bucket.remove(key)
    }

    /// Remove all elements that are "old".
    ///
    /// Old elements no longer fit into defined buckets.
    pub fn spill<E: Extend<(K, u64)>>(&mut self, target: &mut E) {
        target.extend(self.old_bucket.iter().map(
            |(k, v)| (k.clone(), *v)
        ));
        self.old_bucket.clear();
    }

    pub fn clear(&mut self) {
        self.next_bucket.clear();
        self.old_bucket.clear();
        self.buckets.clear();
    }

    /// Get usage of all buckets.
    pub fn detailed_usage(&self) -> Vec<(u64, Option<u64>)> {
        let mut res = Vec::with_capacity(2 + self.bucket_count as usize);

        res.push((self.old_bucket.usage(), None));
        for b in &self.buckets {
            res.push((b.usage(), Some(self.max_bucket_usage)));
        }
        res.push((self.next_bucket.usage(), Some(self.max_bucket_usage)));

        res
    }

    /// Get usage of all buckets.
    pub fn simple_usage(&self) -> Vec<u64> {
        let mut res = Vec::with_capacity(2 + self.bucket_count as usize);

        res.push(self.old_bucket.usage());
        for b in &self.buckets {
            res.push(b.usage());
        }
        res.push(self.next_bucket.usage());

        res
    }

    /// Get total usage.
    pub fn usage(&self) -> u64 {
        let mut res = self.old_bucket.usage();
        for b in &self.buckets {
            res += b.usage();
        }
        res += self.next_bucket.usage();

        res
    }

    /// Find the key in bucket history and remove it from there.
    fn dig_out<Q: ?Sized>(&mut self, key: &Q) -> bool
        where
            K: Borrow<Q>,
            Q: Eq + Hash
    {
        for b in &mut self.buckets {
            if b.remove(key) {
                return true;
            }
        }
        false
    }

    fn burry_bucket(&mut self) {
        let new_bucket = if self.buckets.len() as u64 >= self.bucket_count {
            let mut old = self.buckets.pop_front().unwrap();
            self.old_bucket.extend(old.iter().map(
                |(k, v)| (k.clone(), *v)
            ));
            old.clear();
            old
        } else {
            Bucket::new()
        };

        let mut current_bucket = new_bucket;
        mem::swap(&mut current_bucket, &mut self.next_bucket);

        self.buckets.push_back(current_bucket);
    }
}

#[cfg(test)]
mod bucket_test {
    use super::*;

    #[test]
    fn usage_is_zero() {
        let b = Bucket::<u32>::new();
        assert_eq!(0, b.usage());
    }

    #[test]
    fn store_correct_usage() {
        let mut b = Bucket::new();
        b.insert(3, 2);
        assert_eq!(2, b.usage());
        b.insert(2, 3);
        assert_eq!(5, b.usage());
    }

    #[test]
    fn contains_stored() {
        let mut b = Bucket::new();
        b.insert(3, 2);
        assert!(b.contains(&3));
    }

    #[test]
    fn not_contains_removed() {
        let mut b = Bucket::new();
        b.insert(3, 2);
        b.remove(&3);
        assert!(!b.contains(&3));
        assert_eq!(0, b.usage());
    }

    #[test]
    fn is_iteratable() {
        let mut b = Bucket::new();
        b.insert(3, 2);
        b.insert(1, 1);

        let all = b.iter().map(|(k, _)| *k).collect::<Vec<_>>();
        assert!(all.contains(&3));
        assert!(all.contains(&1));
    }

    #[test]
    fn not_contains_cleared() {
        let mut b = Bucket::new();
        b.insert(3, 2);
        b.clear();
        assert!(!b.contains(&3));
        assert_eq!(0, b.usage());
    }

    #[test]
    fn can_be_extended() {
        let mut b = Bucket::new();
        b.insert(3, 2);

        let mut c = Bucket::new();
        c.insert(1, 1);

        b.extend(c.iter().map(|(k, v)| (*k, *v)));

        assert!(b.contains(&3));
        assert!(b.contains(&1));
        assert_eq!(3, b.usage());

        assert!(c.contains(&1));
        assert_eq!(1, c.usage());
    }
}

#[cfg(test)]
mod history_test {
    use std::collections::BTreeSet;
    use std::hash::Hash;
    use super::*;

    #[test]
    fn spills_oldest() {
        let mut h = History::new(2, 2);
        h.hit(1, 2);
        h.hit(2, 2);
        h.hit(3, 2);
        h.hit(4, 2);

        assert_eq!(vec![2, 2, 2, 2], h.simple_usage());
        assert_eq!(8, h.usage());

        assert_eq!(vec![1], spill_and_get_sorted(&mut h));

        assert_eq!(vec![0, 2, 2, 2], h.simple_usage());
        assert_eq!(6, h.usage());
    }

    #[test]
    fn supports_oversized() {
        let mut h = History::new(2, 2);
        h.hit(1, 4);
        h.hit(2, 5);
        h.hit(3, 6);
        h.hit(4, 7);

        assert_eq!(vec![4, 5, 6, 7], h.simple_usage());
        assert_eq!(22, h.usage());

        assert_eq!(vec![1], spill_and_get_sorted(&mut h));

        assert_eq!(vec![0, 5, 6, 7], h.simple_usage());
        assert_eq!(18, h.usage());
    }

    #[test]
    fn supports_small() {
        let mut h = History::new(2, 1);
        h.hit(1, 1);
        h.hit(2, 1);
        h.hit(3, 1);

        assert_eq!(vec![0, 2, 1], h.simple_usage());
        assert_eq!(3, h.usage());

        h.hit(4, 1);
        h.hit(5, 1);

        assert_eq!(vec![2, 2, 1], h.simple_usage());
        assert_eq!(5, h.usage());

        assert_eq!(vec![1, 2], spill_and_get_sorted(&mut h));

        assert_eq!(vec![0, 2, 1], h.simple_usage());
        assert_eq!(3, h.usage());
    }

    #[test]
    fn digs_out_when_used_again() {
        let mut h = History::new(2, 1);
        h.hit(1, 1);
        h.hit(2, 1);

        h.hit(1, 1);

        assert_eq!(vec![0, 1, 1], h.simple_usage());
        assert_eq!(2, h.usage());
    }

    #[test]
    fn removes_recent() {
        let mut h = History::new(2, 1);
        h.hit(1, 1);
        h.hit(2, 1);
        h.hit(3, 1);

        h.remove(&3);

        assert_eq!(vec![0, 2, 0], h.simple_usage());
        assert_eq!(2, h.usage());
    }

    #[test]
    fn removes_burried() {
        let mut h = History::new(2, 1);
        h.hit(1, 1);
        h.hit(2, 1);
        h.hit(3, 1);

        h.remove(&1);
        h.remove(&2);

        assert_eq!(vec![0, 0, 1], h.simple_usage());
        assert_eq!(1, h.usage());
    }

    #[test]
    fn removes_old() {
        let mut h = History::new(2, 1);
        h.hit(1, 1);
        h.hit(2, 1);
        h.hit(3, 1);
        h.hit(4, 1);

        h.remove(&1);
        h.remove(&2);

        assert_eq!(vec![0, 0, 2], h.simple_usage());
        assert_eq!(2, h.usage());
    }

    fn spill_and_get_sorted<V>(history: &mut History<V>) -> Vec<V> where V: Eq + Hash + Clone + Ord {
        let mut res = BTreeSet::new();
        history.spill(&mut res);

        res.iter().map(|&(ref k, _)| k.clone()).collect()
    }
}
