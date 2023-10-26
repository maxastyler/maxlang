use std::{
    alloc::{alloc, Layout},
    fmt::Debug,
    hash::{BuildHasher, Hasher},
    mem::{align_of, size_of},
    ptr::null_mut,
};

use im::{HashMap, Vector};

use std::alloc::dealloc;

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

#[derive(Clone)]
enum Value {
    Number(f64),
    Object(*mut HeapElement),
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
    List(Vector<Value>),
}

impl Object {
    fn add_children_to_grey_list(&self, grey_list: &mut Vec<*mut HeapElement>) {
        match self {
            Object::HashMap(h) => {
                for (k, v) in h.iter() {
                    if let Value::Object(o) = k {
                        grey_list.push(*o);
                    }
                    if let Value::Object(o) = v {
                        grey_list.push(*o);
                    }
                }
            }
            Object::List(l) => {
                for v in l.iter() {
                    if let Value::Object(o) = v {
                        grey_list.push(*o);
                    }
                }
            }
        }
    }
}

pub struct HeapElement {
    object: Object,
    marked: bool,
    next: *mut HeapElement,
}

pub struct Heap {
    start: *mut HeapElement,
}

impl Debug for Heap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("hi");
        Ok(())
    }
}

impl Heap {
    fn new() -> Self {
        Self { start: null_mut() }
    }

    fn trace(&self, roots: Vec<*mut HeapElement>) {
        let mut grey_list = roots;
        while let Some(ptr) = grey_list.pop() {
            let heap_elem = unsafe { ptr.as_mut().unwrap() };
            heap_elem.marked = true;
            (*heap_elem)
                .object
                .add_children_to_grey_list(&mut grey_list);
        }
    }

    fn sweep(&mut self) {
        let mut prev = null_mut();
        let mut current = self.start;
        while !current.is_null() {
            if unsafe { (*current).marked } {
                unsafe { (*current).marked = false };
                prev = current;
                current = unsafe { (*current).next };
            } else {
                let unreached = current;
                current = unsafe { (*current).next };
                if prev.is_null() {
                    self.start = current;
                } else {
                    unsafe {
                        (*prev).next = current;
                    }
                }
                unsafe {
                    unreached.drop_in_place();
                    dealloc(
                        unreached.cast(),
                        Layout::from_size_align_unchecked(
                            size_of::<HeapElement>(),
                            align_of::<HeapElement>(),
                        ),
                    )
                }
            }
        }
    }

    fn alloc(&mut self, object: Object) -> Value {
        let new_ptr = unsafe {
            alloc(Layout::from_size_align_unchecked(
                size_of::<HeapElement>(),
                align_of::<HeapElement>(),
            ))
        }
        .cast();
        unsafe {
            (*new_ptr) = HeapElement {
                object,
                marked: false,
                next: self.start,
            };
            self.start = new_ptr;
        };
        Value::Object(new_ptr)
    }
}

#[cfg(test)]
mod test {
    use im::Vector;

    use super::Heap;

    #[test]
    fn heap_works() {
        let mut heap = Heap::new();
        let o = heap.alloc(super::Object::List(Vector::from_iter(
            [2, 3, 4].iter().map(|x| super::Value::Number(*x as f64)),
        )));
        // heap.trace(vec![match o {
        //     crate::heap::Value::Object(o) => o,
        //     _ => unreachable!(),
        // }]);
	heap.trace(vec![]);
        heap.sweep();
        assert!(heap.start.is_null());
    }
}
