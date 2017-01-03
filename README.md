# CLOCK Cache

[![Build Status](https://travis-ci.org/jeromefroe/clock_cache.svg?branch=master)](https://travis-ci.org/jeromefroe/clock_cache)
[![Coverage Status](https://coveralls.io/repos/github/jeromefroe/clock_cache/badge.svg?branch=master)](https://coveralls.io/github/jeromefroe/clock_cache?branch=master)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](https://raw.githubusercontent.com/jeromefroe/clock_cache/master/LICENSE)

An implemenation of a CLOCK cache as first described in
[A Paging Experiment with the Multics System] (http://multicians.org/paging-experiment.pdf).

## Example

Below is a simple example of how to instantiate a CLOCK cache.

```rust,no_run
extern crate clock_cache;

use clock_cache::ClockCache;

fn main() {
        let mut cache = ClockCache::new(2);
        cache.put("apple", "red");
        cache.put("banana", "yellow");

        assert_eq!(*cache.get(&"apple").unwrap(), "red");
        assert_eq!(*cache.get(&"banana").unwrap(), "yellow");
        assert!(cache.get(&"pear").is_none());

        cache.put("pear", "green");

        assert_eq!(*cache.get(&"pear").unwrap(), "green");
        assert_eq!(*cache.get(&"banana").unwrap(), "yellow");
        assert!(cache.get(&"apple").is_none());
}
```