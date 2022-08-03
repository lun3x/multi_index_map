# MultiIndexMap

Rust library useful for storing structs that needs to be accessed through various different indexes of the fields of the struct. Inspired by [C++/Boost Multi-index Containers](https://www.boost.org/doc/libs/1_79_0/libs/multi_index/doc/index.html) but redesigned for a more idiomatic Rust API.

Initial implementation supports:
* Hashed indexes using FxHashMap from [rustc-hash](https://github.com/rust-lang/rustc-hash)
* Sorted indexes using BTreeMap from [std::collections](https://doc.rust-lang.org/std/collections/struct.BTreeMap.html)
* Unindexed fields.
* Iterators for each indexed field.
* Iterators for the underlying backing storage (Slab).

# Performance characteristics
* Hashed indexes are constant-time retrieval. (FxHashMap + Slab).
* Sorted indexes are logarithmic time retrieval. (BTreeMap + Slab).
* Iteration over hashed index is same as FxHashMap, plus a retrieval from the backing storage for each element.
* Iteration over ordered index is same as BTreeMap, plus a retrieval from the backing storage for each element.
* Iteration over the backing store is the same as Slab, so contiguous memory but with potentially vacant slots.
* Insertion, removal, and modification complexity grows as the number of indexed fields grow. All fields must be inserted into / removed from / reinserted into during these operations so these are slower.
* Modification of unindexed fields through unsafe mut methods is the same as regular retrieval time.

# How to use

This crate provides a derive macro `MultiIndexMap`, which when applied to the struct representing an element will generate a map to store and access these elements.
Annotations are used to specify which fields to index. Currently `hashed_unique` and `ordered_unique` are supported.
The element must implement `Clone`.

## Example

```rust
#[derive(MultiIndexMap, Clone)]
struct Order {
    #[multi_index(hashed_unique)]
    order_id: u32,
    #[multi_index(ordered_unique)]
    timestamp: u64,
    #[multi_index(hashed_non_unique)]
    trader_name: String,
}

fn main() {
    let order1 = Order {
        order_id: 1,
        timestamp: 1656145181,
        trader_name: "JohnDoe".into(),
    };

    let order2 = Order {
        order_id: 2,
        timestamp: 1656145182,
        trader_name: "JohnDoe".into(),
    };

    let mut map = MultiIndexOrderMap::default();

    map.insert(order1);
    map.insert(order2);

    let order1_ref = map.get_by_order_id(&1).unwrap();
    assert_eq!(order1_ref.timestamp, 1656145181);

    let order2_ref = map.modify_by_order_id(&2, |o| {
        o.timestamp = 1656145183;
        o.order_id = 42;
    }).unwrap();
    assert_eq!(order2_ref.timestamp, 1656145183);
    assert_eq!(order2_ref.order_id, 42);
    assert_eq!(order2_ref.trader_name, "JohnDoe".into());

    let orders = map.remove_by_trader_name("JohnDoe".into());
    assert_eq!(orders.len(), 2);
    

    // See examples directory for more in depth usage.
}
```

# Under the hood

The above example will generate the following MultiIndexMap and associated Iterators.
The `Order`s are stored in a `Slab`, in contiguous memory, which allows for fast lookup and quick iteration. 
A lookup table is created for each indexed field, which maps the index key to a index in the `Slab`.
The exact type used for these depends on the annotations.
For `hashed_unique` and `hashed_non_unique` a `FxHashMap` is used, for `ordered_unique` a BTreeMap is used.
* When inserting an element, we add it to the backing store, then add elements to each lookup table pointing to the index in the backing store.
* When retrieving elements for a given key, we lookup the key in the lookup table, then retrieve the item at that index in the backing store.
* When removing an element for a given key, we do the same, but we then must also remove keys from all the other lookup tables before returning the element.
* When iterating over an index, we use the default iterators for the lookup table, then simply retrieve the element at the given index in the backing store.
* When modifying an element, we lookup the element through the given key, then apply the closure to modify the element in-place. We then return a reference to the modified element.
We must then update all the lookup tables to account for any changes to indexed fields.
If we only want to modify an unindexed field then it is much faster to just mutate that field directly.
This is why the unsafe methods are provided. These can be used to modify unindexed fields quickly, but must not be used to modify indexed fields.


```rust
struct MultiIndexOrderMap {
    _store: slab::Slab<Order>,
    _order_id_index: rustc_hash::FxHashMap<u32, usize>,
    _timestamp_index: std::collections::BTreeMap<u64, usize>,
    _trader_name_index: rustc_hash::FxHashMap<u32, usize>,
}

struct MultiIndexOrderMapOrderIdIter<'a> {
    _store_ref: &'a slab::Slab<Order>,
    _iter: std::collections::hash_map::Iter<'a, u32, usize>,
}

struct MultiIndexOrderMapTraderNameIter<'a> {
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
    unsafe fn get_mut_by_order_id(&mut self) -> Option<&mut Order>;
    unsafe fn get_mut_by_timestamp(&mut self) -> Option<&mut Order>;
    fn modify_by_order_id(&mut self, f: impl FnOnce(&mut Order)) -> Option<&Order>;
    fn modify_by_timestamp(&mut self, f: impl FnOnce(&mut Order)) -> Option<&Order>;
    fn remove_by_order_id(&mut self) -> Option<Order>;
    fn remove_by_timestamp(&mut self) -> Option<Order>;
    fn remove_by_trader_name(&mut self) -> Vec<Order>;
    fn iter(&self) -> slab::Iter<Order>;
    unsafe fn iter_mut(&mut self) -> slab::IterMut<Order>;
    fn iter_by_order_id(&self) -> MultiIndexOrderMapOrderIdIter;
    fn iter_by_timestamp(&self) -> MultiIndexOrderMapTimestampIter;
    fn iter_by_trader_name(&self) -> MultiIndexOrderMapTraderNameIter;
}
```


# Future work
* More index kinds, `ordered_non_unique` is very useful for retrieving all elements matching the given index key. Potentially a vector-map style lookup table would be very quick for small tables with integer indexes.
* Implement [clever tricks](https://www.boost.org/doc/libs/1_36_0/libs/multi_index/doc/performance.html) used in boost::multi_index_containers to improve performance.
