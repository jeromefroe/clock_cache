// MIT License

// Copyright (c) 2017 Jerome Froelich

// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! An implemenation of a CLOCK cache as first described in
//! [A Paging Experiment with the Multics System] (http://multicians.org/paging-experiment.pdf).
//!
//! ## Example
//!
//! ```rust,no_run
//! extern crate clock_cache;
//!
//! use clock_cache::ClockCache;
//!
//! fn main() {
//!         let mut cache = ClockCache::new(2);
//!         cache.put("apple", "red");
//!         cache.put("banana", "yellow");
//!
//!         assert_eq!(*cache.get(&"apple").unwrap(), "red");
//!         assert_eq!(*cache.get(&"banana").unwrap(), "yellow");
//!         assert!(cache.get(&"pear").is_none());
//!
//!         cache.put("pear", "green");
//!
//!         assert_eq!(*cache.get(&"pear").unwrap(), "green");
//!         assert_eq!(*cache.get(&"banana").unwrap(), "yellow");
//!         assert!(cache.get(&"apple").is_none());
//! }
//! ```


extern crate bit_vec;

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use bit_vec::BitVec;

// Struct used to hold a reference to a key
struct KeyRef<K> {
    k: *const K,
}

impl<K: Hash> Hash for KeyRef<K> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        unsafe { (*self.k).hash(state) }
    }
}

impl<K: PartialEq> PartialEq for KeyRef<K> {
    fn eq(&self, other: &KeyRef<K>) -> bool {
        unsafe { (*self.k).eq(&*other.k) }
    }
}

impl<K: Eq> Eq for KeyRef<K> {}

struct ClockEntry<K, V> {
    key: K,
    val: V,
}

impl<K, V> ClockEntry<K, V> {
    fn new(key: K, val: V) -> Self {
        ClockEntry {
            key: key,
            val: val,
        }
    }
}

/// A Clock Cache
pub struct ClockCache<K, V> {
    map: HashMap<KeyRef<K>, usize>,
    entries: Vec<ClockEntry<K, V>>,
    bits: BitVec,
    cap: usize,
    idx: usize,
}

impl<K: Hash + Eq, V> ClockCache<K, V> {
    /// Create a new ClockCache that holds at most `cap` items.
    ///
    /// # Example
    ///
    /// ```
    /// use clock_cache::ClockCache;
    /// let mut cache: ClockCache<isize, &str> = ClockCache::new(10);
    /// ```
    pub fn new(cap: usize) -> ClockCache<K, V> {
        ClockCache {
            map: HashMap::with_capacity(cap),
            entries: Vec::with_capacity(cap),
            bits: BitVec::from_fn(cap, |_| false),
            cap: cap,
            idx: 0,
        }
    }

    /// Put a key-value pair into the cache. If the key already exists update its value.
    ///
    /// # Example
    ///
    /// ```
    /// use clock_cache::ClockCache;
    /// let mut cache = ClockCache::new(2);
    ///
    /// cache.put(1, "a");
    /// cache.put(2, "b");
    /// assert_eq!(cache.get(&1), Some(&"a"));
    /// assert_eq!(cache.get(&2), Some(&"b"));
    /// ```
    pub fn put(&mut self, k: K, v: V) {
        // check if the key is already in the cache
        match self.map.get(&KeyRef { k: &k }) {
            Some(idx) => {
                self.entries.get_mut(*idx).map(|entry| entry.val = v);
                return;
            }
            None => (),
        };

        let entry = if self.entries.len() < self.cap {
            // if entries is not full yet, push a new entry onto the end
            self.entries.push(ClockEntry::new(k, v));
            self.entries.get_mut(self.idx).unwrap()
        } else {
            // if entries is full, find and use the first entry with its usage bit set to false
            let mut usage_bit = self.bits.get(self.idx).unwrap();

            while usage_bit {
                self.bits.set(self.idx, false);
                self.idx = (self.idx + 1) % self.cap;
                usage_bit = self.bits.get(self.idx).unwrap();
            }

            self.bits.set(self.idx, true);

            let entry = self.entries.get_mut(self.idx).unwrap();

            let old_key = KeyRef { k: &entry.key };
            self.map.remove(&old_key);

            entry.key = k;
            entry.val = v;
            entry
        };

        let key = KeyRef { k: &entry.key };
        self.map.insert(key, self.idx);

        self.idx = (self.idx + 1) % self.cap;
    }

    /// Return the value corresponding to the key in the cache or `None` if it is not
    /// present in the cache. Update the key's usage bit if it exists.
    ///
    /// # Example
    ///
    /// ```
    /// use clock_cache::ClockCache;
    /// let mut cache = ClockCache::new(2);
    ///
    /// cache.put(1, "a");
    /// cache.put(2, "b");
    /// cache.put(2, "c");
    /// cache.put(3, "d");
    ///
    /// assert_eq!(cache.get(&1), None);
    /// assert_eq!(cache.get(&2), Some(&"c"));
    /// assert_eq!(cache.get(&3), Some(&"d"));
    /// ```
    pub fn get<'a>(&'a mut self, k: &K) -> Option<&'a V> {
        let key = KeyRef { k: k };
        match self.map.get(&key) {
            None => None,
            Some(idx) => {
                self.bits.set(*idx, true);
                Some(self.entries.get(*idx).map(|entry| &entry.val).unwrap())
            }
        }
    }

    /// Return the value corresponding to the key in the cache or `None` if it is not
    /// present in the cache. Unlike `get`, `peek` does not update the key's usage bit.
    ///
    /// # Example
    ///
    /// ```
    /// use clock_cache::ClockCache;
    /// let mut cache = ClockCache::new(2);
    ///
    /// cache.put(1, "a");
    /// cache.put(2, "b");
    ///
    /// assert_eq!(cache.peek(&1), Some(&"a"));
    /// assert_eq!(cache.peek(&2), Some(&"b"));
    /// ```
    pub fn peek<'a>(&'a mut self, k: &K) -> Option<&'a V> {
        let key = KeyRef { k: k };
        match self.map.get(&key) {
            None => None,
            Some(idx) => Some(self.entries.get(*idx).map(|entry| &entry.val).unwrap()),
        }
    }

    /// Return a bool indicating whether the given key is in the cache. Does not update the
    /// key's usage bit.
    ///
    /// # Example
    ///
    /// ```
    /// use clock_cache::ClockCache;
    /// let mut cache = ClockCache::new(2);
    ///
    /// cache.put(1, "a");
    /// cache.put(2, "b");
    /// cache.put(3, "c");
    ///
    /// assert!(!cache.contains(&1));
    /// assert!(cache.contains(&2));
    /// assert!(cache.contains(&3));
    /// ```
    pub fn contains(&self, k: &K) -> bool {
        let key = KeyRef { k: k };
        self.map.contains_key(&key)
    }

    /// Remove a key from the cache and return a boolean indicating whether the key was in the
    /// cache or not.
    ///
    /// # Example
    ///
    /// ```
    /// use clock_cache::ClockCache;
    /// let mut cache = ClockCache::new(2);
    ///
    /// cache.put(2, "a");
    ///
    /// assert!(!cache.pop(&1));
    /// assert!(cache.pop(&2));
    /// assert!(!cache.pop(&2));
    /// assert_eq!(cache.len(), 0);
    /// ```
    pub fn pop(&mut self, k: &K) -> bool {
        let key = KeyRef { k: k };
        match self.map.remove(&key) {
            None => false,
            Some(_) => true,
        }
    }

    /// Return the number of key-value pairs that are currently in the the cache.
    ///
    /// # Example
    ///
    /// ```
    /// use clock_cache::ClockCache;
    /// let mut cache = ClockCache::new(2);
    /// assert_eq!(cache.len(), 0);
    ///
    /// cache.put(1, "a");
    /// assert_eq!(cache.len(), 1);
    ///
    /// cache.put(2, "b");
    /// assert_eq!(cache.len(), 2);
    ///
    /// cache.put(3, "c");
    /// assert_eq!(cache.len(), 2);
    /// ```
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Return the maximum number of key-value pairs the cache can hold.
    ///
    /// # Example
    ///
    /// ```
    /// use clock_cache::ClockCache;
    /// let mut cache: ClockCache<isize, &str> = ClockCache::new(2);
    /// assert_eq!(cache.cap(), 2);
    /// ```
    pub fn cap(&self) -> usize {
        self.cap
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;
    use super::ClockCache;

    fn assert_opt_eq<V: PartialEq + Debug>(opt: Option<&V>, v: V) {
        assert!(opt.is_some());
        assert_eq!(opt.unwrap(), &v);
    }

    #[test]
    fn test_put_and_get() {
        let mut cache = ClockCache::new(2);

        cache.put("apple", "red");
        cache.put("banana", "yellow");

        assert_eq!(cache.cap(), 2);
        assert_eq!(cache.len(), 2);
        assert_opt_eq(cache.get(&"apple"), "red");
        assert_opt_eq(cache.get(&"banana"), "yellow");
    }

    #[test]
    fn test_put_update() {
        let mut cache = ClockCache::new(1);

        cache.put("apple", "red");
        cache.put("apple", "green");

        assert_eq!(cache.len(), 1);
        assert_opt_eq(cache.get(&"apple"), "green");
    }

    #[test]
    fn test_peek() {
        let mut cache = ClockCache::new(2);

        cache.put("apple", "red");
        cache.put("banana", "yellow");

        assert_opt_eq(cache.peek(&"banana"), "yellow");
        assert_opt_eq(cache.peek(&"apple"), "red");

        cache.put("pear", "green");

        assert!(cache.peek(&"apple").is_none());
        assert_opt_eq(cache.peek(&"banana"), "yellow");
        assert_opt_eq(cache.peek(&"pear"), "green");
    }

    #[test]
    fn test_contains() {
        let mut cache = ClockCache::new(2);

        cache.put("apple", "red");
        cache.put("banana", "yellow");
        cache.put("pear", "green");

        assert!(!cache.contains(&"apple"));
        assert!(cache.contains(&"banana"));
        assert!(cache.contains(&"pear"));
    }

    #[test]
    fn test_pop() {
        let mut cache = ClockCache::new(2);

        cache.put("apple", "red");
        cache.put("banana", "yellow");

        assert!(cache.pop(&"apple"));
        assert!(cache.pop(&"banana"));
        assert!(!cache.pop(&"apple"));
        assert!(!cache.pop(&"apple"));
        assert_eq!(cache.len(), 0);
    }
}
