extern crate bytecache;

use std::io;
use std::io::prelude::*;
use bytecache::mem::MemCache;

fn print_state(memcache: &MemCache<String>) {
    println!("mem usage: {:?}, buckets: {:?}", memcache.usage(), memcache.detailed_usage());
}

fn main() {
    let mut memcache = MemCache::new(40);

    println!("cache max: 40 bytes");
    println!("commands: set <key> <value>, get <key>");

    print_state(&memcache);

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        match line {
            Ok(line) => {
                if line.starts_with("set") {
                    let kv = (&line["set".len()..]).trim_left().splitn(2, " ").collect::<Vec<_>>();
                    memcache.set(kv[0].to_string(), kv[1].as_bytes().into());
                } else if line.starts_with("get") {
                    let key = (&line["get".len()..]).trim_left();
                    match memcache.get(key.to_string()) {
                        Some(v) => println!("{}", String::from_utf8_lossy(v)),
                        None => println!("not found"),
                    }
                }
            },
            _ => break,
        }

        print_state(&memcache);
    }
}
