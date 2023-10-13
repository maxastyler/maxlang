pub enum Object {
    Number(f64)
}

pub struct HeapElement {
    object: Object,
    next: *mut HeapElement
}


pub struct Heap {
    start: Option<*mut Object>,
}
