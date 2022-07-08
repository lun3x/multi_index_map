# MultiIndexMap

Rust library useful for storing structs that needs to be accessed through various different indexes of the fields of the struct. Inspired by [C++/Boost Multi-index Containers](https://www.boost.org/doc/libs/1_79_0/libs/multi_index/doc/index.html) but redesigned for a more idiomatic Rust API.

Initial implementation supports:
* Hashed indexes using FxHashMap from [rustc-hash](https://github.com/rust-lang/rustc-hash)
* Sorted indexes using BTreeMap from [std::collections](https://doc.rust-lang.org/std/collections/struct.BTreeMap.html)
* Unindexed fields.
* Iterators for each indexed field.
* Iterators for the underlying storage.

# How to use

This crate provides a derive macro `MultiIndexMap`, which when applied to the struct representing an element will generate a map to store and access these elements.
The element must implement Clone.

## Example

```rust
#[derive(MultiIndexMap, Clone)]
struct Order {
    #[multi_index(hashed_unique)]
    order_id: u32,
    #[multi_index(ordered_unique)]
    timestamp: u64,
    trader_name: String,
}
```

This will generate the following MultiIndexMap:

```rust
struct MultiIndexOrderMap {
    _store: slab::Slab<Order>,
    _order_id_index: rustc_hash::FxHashMap<u32, usize>,
    _timestamp_index: std::collections::BTreeMap<u64, usize>,
}

struct MultiIndexOrderMapOrderIdIter<'a> {
    _store_ref: &'a slab::Slab<Order>,
    _iter: std::collections::hash_map::Iter<'a, u32, usize>,
}

struct MultiIndexOrderMapTimestampIter<'a> {
    _store_ref: &'a slab::Slab<Order>,
    _iter: std::collections::btree_map::Iter<'a, u64, usize>,
}

impl MultiIndexOrderMap {
    fn insert(&mut self, elem: Order);
    fn get_by_order_id(&self) -> Option<&Order>;
    fn get_by_timestamp(&self) -> Option<&Order>;
    unsafe fn get_mut_by_order_id(&mut self) -> Option<&Order>;
    unsafe fn get_mut_by_timestamp(&mut self) -> Option<&Order>;
    fn modify_by_order_id(&mut self, f: impl FnOnce(&mut Order)) -> Option<&Order>;
    fn modify_by_timestamp(&mut self, f: impl FnOnce(&mut Order)) -> Option<&Order>;
    fn remove_by_order_id(&mut self) -> Option<Order>;
    fn remove_by_timestamp(&mut self) -> Option<Order>;
    fn iter(&self) -> slab::Iter<Order>;
    unsafe fn iter_mut(&mut self) -> slab::IterMut<Order>;
    fn iter_by_order_id(&self) -> MultiIndexOrderMapOrderIdIter;
    fn iter_by_timestamp(&self) -> MultiIndexOrderMapTimestampIter;  
}
```
