use std::collections::VecDeque;
use std::collections::HashSet;
use std::collections::hash_set;
use std::hash::Hash;
use std::borrow::Borrow;
use std::ops::Deref;
use std::mem;

use RequiredBytes;

pub struct Bucket<K> {
    items: HashSet<K>,
    usage: u64,
}

/// Bucket wraps a HashSet and requires items to implement `RequiredBytes`.
///
/// At any point it is possible to query bucket for `usage` and check how many
/// "bytes" it uses. Of course, the measuring in "bytes" does not matter, the `usage`
/// can be anything.
impl<K> Bucket<K>
    where K: Hash + Eq
{
    pub fn new() -> Bucket<K> {
        Bucket::<K> {
            items: HashSet::new(),
            usage: 0,
        }
    }

    #[inline(always)]
    pub fn contains<Q: ?Sized>(&self, value: &Q) -> bool
        where
            K: Borrow<Q>,
            Q: Hash + Eq
    {
        self.items.contains(value)
    }

    pub fn insert(&mut self, value: K) -> bool
        where
            K: RequiredBytes
    {
        self.usage += value.required_bytes();
        self.items.insert(value)
    }

    pub fn remove<Q: ?Sized>(&mut self, value: &Q) -> bool
        where
            K: Borrow<Q>,
            Q: Hash + Eq + RequiredBytes
    {
        self.usage -= value.required_bytes();
        self.items.remove(value)
    }

    pub fn iter(&self) -> hash_set::Iter<K> {
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

impl<K> Extend<K> for Bucket<K> where K: RequiredBytes + Eq + Hash {
    fn extend<T>(&mut self, iter: T)
        where
            T: IntoIterator<Item=K>
    {
        let iter = iter.into_iter();
        if let (_, Some(len)) = iter.size_hint() {
            self.items.reserve(len);
        }
        for v in iter {
            self.insert(v);
        }
    }
}

#[cfg(test)]
mod bucket_test {
    use super::*;
    use RequiredBytes;

    #[derive(Hash, Eq, PartialEq, Debug, Copy, Clone)]
    struct Val {
        num: i32,
        usage: u64,
    }

    impl RequiredBytes for Val {
        fn required_bytes(&self) -> u64 {
            self.usage
        }
    }

    #[test]
    fn usage_is_zero() {
        let b = Bucket::<u32>::new();
        assert_eq!(0, b.usage());
    }

    #[test]
    fn store_correct_usage() {
        let mut b = Bucket::new();
        b.insert(Val { num: 3, usage: 2 });
        assert_eq!(2, b.usage());
        b.insert(Val { num: 2, usage: 3 });
        assert_eq!(5, b.usage());
    }

    #[test]
    fn contains_stored() {
        let mut b = Bucket::new();
        b.insert(Val { num: 3, usage: 2 });
        assert!(b.contains(&Val { num: 3, usage: 2 }));
    }

    #[test]
    fn not_contains_removed() {
        let mut b = Bucket::new();
        b.insert(Val { num: 3, usage: 2 });
        b.remove(&Val { num: 3, usage: 2 });
        assert!(!b.contains(&Val { num: 3, usage: 2 }));
        assert_eq!(0, b.usage());
    }

    #[test]
    fn is_iteratable() {
        let mut b = Bucket::new();
        b.insert(Val { num: 3, usage: 2 });
        b.insert(Val { num: 1, usage: 1 });

        let all = b.iter().map(|v| *v).collect::<Vec<_>>();
        assert!(all.contains(&Val { num: 3, usage: 2 }));
        assert!(all.contains(&Val { num: 1, usage: 1 }));
    }

    #[test]
    fn not_contains_cleared() {
        let mut b = Bucket::new();
        b.insert(Val { num: 3, usage: 2 });
        b.clear();
        assert!(!b.contains(&Val { num: 3, usage: 2 }));
        assert_eq!(0, b.usage());
    }

    #[test]
    fn can_be_extended() {
        let mut b = Bucket::new();
        b.insert(Val { num: 3, usage: 2 });

        let mut c = Bucket::new();
        c.insert(Val { num: 1, usage: 1 });

        b.extend(c.iter().cloned());

        assert!(b.contains(&Val { num: 3, usage: 2 }));
        assert!(b.contains(&Val { num: 1, usage: 1 }));
        assert_eq!(3, b.usage());

        assert!(c.contains(&Val { num: 1, usage: 1 }));
        assert_eq!(1, c.usage());
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
    where K: Eq + Hash + Copy
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
    pub fn refresh(&mut self, key: K)
        where
            K: Eq + Hash + RequiredBytes
    {
        if self.next_bucket.contains(&key) {
            return;
        }

        self.dig_out(&key);

        self.next_bucket.insert(key);

        if self.next_bucket.usage() >= self.max_bucket_usage {
            self.push_bucket();
        }
    }

    /// Remove key from history.
    pub fn remove<Q: ?Sized>(&mut self, key: &Q)
        where
            K: Borrow<Q>,
            Q: Eq + Hash + RequiredBytes
    {
        if self.next_bucket.remove(key) {
            return;
        }

        self.dig_out(key);
    }

    /// Remove all elements that are "old".
    ///
    /// Old elements no longer fit into defined buckets.
    pub fn spill<E: Extend<K>>(&mut self, target: &mut E) {
        target.extend(self.old_bucket.iter().cloned());
        self.old_bucket.clear();
    }

    /// Get usage of all buckets.
    pub fn detailed_usage(&self) -> Vec<u64> {
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
    fn dig_out<Q: ?Sized>(&mut self, key: &Q)
        where
            K: Borrow<Q>,
            Q: Eq + Hash + RequiredBytes
    {
        for b in &mut self.buckets {
            if b.remove(key) {
                return;
            }
        }
    }

    fn push_bucket(&mut self)
        where K: RequiredBytes
    {
        let new_bucket = if self.buckets.len() as u64 >= self.bucket_count {
            let mut old = self.buckets.pop_front().unwrap();
            self.old_bucket.extend(old.iter().cloned());
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
mod test {
    use super::*;

    #[test]
    fn store_and_get() {

    }
}
