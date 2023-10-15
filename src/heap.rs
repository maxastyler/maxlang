use std::hash::{BuildHasher, Hasher};

use im::HashMap;

use crate::value::Value;

struct FNVHasher(u32);

impl Hasher for FNVHasher {
    fn finish(&self) -> u64 {
        self.0 as u64
    }

    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 ^= *b as u32;
            self.0 *= 16777619;
        }
    }
}

struct FNVBuilder;

impl BuildHasher for FNVBuilder {
    type Hasher = FNVHasher;

    fn build_hasher(&self) -> Self::Hasher {
        FNVHasher(2166136261)
    }
}

fn new_hash_map() -> HashMap<Value, Value, FNVBuilder> {
    HashMap::with_hasher(FNVBuilder)
}

pub enum Object {
    HashMap(HashMap<Value, Value, FNVBuilder>),
}

pub struct HeapElement {
    object: Object,
    marked: bool,
    next: *mut HeapElement,
}

pub struct Heap {
    start: Option<*mut Object>,
}
