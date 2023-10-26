use std::ptr::{null, null_mut};

enum Node<K, V> {
    Chunk(Chunk<K, V>),
    Leaf(K, V),
    MultiLeaf(usize, *mut (K, V)),
}

struct Chunk<K, V> {
    filled: u64,
    length: u8,
    start: *mut Node<K, V>,
}

impl<K, V> Chunk<K, V> {
    fn insert(&self, key: K, hash: u64, value: V, level: u8) -> Self {
	let element = ((hash >> (6 * level)) & (1<<6-1)) as usize;
	if (self.filled & (1 << element)) == 0 {
	    
	}
        Self {
            filled: todo!(),
            length: todo!(),
            start: todo!(),
        }
    }
}

/// A HashMap is a HAMT (hash array mapped trie)
/// This is a structure with
pub struct HashMap<K, V>(Chunk<K, V>);
impl<K, V> HashMap<K, V> {
    pub fn new() -> Self {
        Self(Chunk {
            filled: 0,
            length: 0,
            start: null_mut(),
        })
    }

    pub fn insert(&self, key: K, value: V) -> Self {
        Self()
    }
}

#[cfg(test)]
mod test {
    use super::HashMap;

    #[test]
    fn test_hashmap() {
        let x: HashMap<i32, i32> = HashMap::new();
    }
}
