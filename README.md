# MultiIndexMap [![Tests](https://github.com/lun3x/multi_index_map/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/lun3x/multi_index_map/actions/workflows/ci.yml)

[Also available on crates.io.](https://crates.io/crates/multi_index_map)

Rust library useful for storing structs that needs to be accessed through various different indexes of the fields of the struct. Inspired by [C++/Boost Multi-index Containers](https://www.boost.org/doc/libs/1_79_0/libs/multi_index/doc/index.html) but redesigned for a more idiomatic Rust API.

Current implementation supports:
* Hashed indexes using HashMap from [std::collections](https://doc.rust-lang.org/std/collections/struct.HashMap.html)
* Sorted indexes using BTreeMap from [std::collections](https://doc.rust-lang.org/std/collections/struct.BTreeMap.html).
* Unique and non-unique indexes.
* Unindexed fields.
* Iterators for each indexed field.
* Iterators for the underlying backing storage.

# Performance characteristics
## Unique Indexes
* Hashed index retrievals are constant-time. (HashMap + Slab).
* Sorted indexes retrievals are logarithmic-time. (BTreeMap + Slab).
* Iteration over hashed index is same as HashMap, plus a retrieval from the backing storage for each element.
* Iteration over ordered index is same as BTreeMap, plus a retrieval from the backing storage for each element.
* Iteration over the backing store is the same as Slab, so contiguous memory but with potentially vacant slots.
* Insertion, removal, and modification complexity grows as the number of indexed fields grow. All indexes must be updated during these operations so these are slower.
* Modification of unindexed fields through get_mut_by_ methods is the same as regular retrieval time.
* Insertion such that uniqueness would be violated does not mutate the map, instead the element is returned to the user wrapped in an Err variant.

## Non-Unique Indexes
* Hashed index retrievals are still constant-time with the total number of elements, but linear-time with the number of matching elements. (HashMap + (Slab * num_matches)).
* Sorted indexes retrievals are still logarithmic-time with total number of elements, but linear-time with the number of matching elements. (BTreeMap + (Slab * num_matches)).
* Each equal range of any non-unique index is stored as a BTreeSet, which we must iterate through the length of when retrieving all matching elements, and also when iterating over the whole index.

# Default Hasher
* The feature `rustc-hash` is enabled by default. It will set the default hash as [`rustc-hash`](https://github.com/rust-lang/rustc-hash/).
* The hash can always be changed by specifying a `BuildHasher` implementation in the `multi_index_hash` attribute, eg. `#[multi_index_hash(ahash::RandomState)]`.
* With default features disabled the default hash will be the standard library default (currently `SipHash`). Default features can be disabled in `Cargo.toml` like so:

```multi_index_map = { version = "*", default-features = false }```

# How to use

* This crate provides a derive macro `MultiIndexMap`, which when applied to the struct representing an element will generate a map to store and access these elements.
* Annotations are used to specify which fields to index. Currently `hashed_unique`, `hashed_non_unique`, `ordered_unique`, and `ordered_non_unique` are supported.
* The types of all indexed fields must implement `Clone`.
* Optionally, `multi_index_derive` can be used to derive traits on the generated MultiIndexMap, eg. `#[multi_index_derive(Clone, Debug)]`
See `examples/main.rs` for more details.

## Example

```rust
use multi_index_map::MultiIndexMap;

#[derive(MultiIndexMap, Debug)]
#[multi_index_derive(Debug)]
#[multi_index_hash(rustc_hash::FxBuildHasher)]
struct Order {
    #[multi_index(hashed_unique)]
    order_id: u32,
    #[multi_index(ordered_unique)]
    timestamp: u64,
    #[multi_index(hashed_non_unique)]
    trader_name: String,
    filled: bool,
    volume: u64,
}

fn main() {
    let order1 = Order {
        order_id: 1,
        timestamp: 1656145181,
        trader_name: "JohnDoe".into(),
        filled: false,
        volume: 100,
    };

    let order2 = Order {
        order_id: 2,
        timestamp: 1656145182,
        trader_name: "JohnDoe".into(),
        filled: false,
        volume: 100,
    };

    let mut map = MultiIndexOrderMap::default();

    map.try_insert(order1).unwrap();
    map.insert(order2);

    let orders = map.get_by_trader_name(&"JohnDoe".to_string());
    assert_eq!(orders.len(), 2);
    println!("Found 2 orders for JohnDoe: [{orders:?}]");

    let order1_ref = map.get_by_order_id(&1).unwrap();
    assert_eq!(order1_ref.timestamp, 1656145181);

    let order2_ref = map
        .modify_by_order_id(&2, |o| {
            o.timestamp = 1656145183;
            o.order_id = 42;
        })
        .unwrap();

    assert_eq!(order2_ref.timestamp, 1656145183);
    assert_eq!(order2_ref.order_id, 42);
    assert_eq!(order2_ref.trader_name, "JohnDoe".to_string());

    let order2_ref = map
        .update_by_order_id(&42, |filled: &mut bool, volume: &mut u64| {
            *filled = true;
            *volume = 0;
        })
        .unwrap();
    assert_eq!(order2_ref.filled, true);
    assert_eq!(order2_ref.volume, 0);

    let orders = map.get_by_trader_name(&"JohnDoe".to_string());
    assert_eq!(orders.len(), 2);
    println!("Found 2 orders for JohnDoe: [{orders:?}]");

    let orders = map.remove_by_trader_name(&"JohnDoe".to_string());
    for (_idx, order) in map.iter() {
        assert_eq!(order.trader_name, "JohnDoe");
    }
    assert_eq!(orders.len(), 2);

    println!("{map:?}");

    // See examples and tests directories for more in depth usage.
}
```

# Under the hood

The above example will generate the following MultiIndexMap and associated Iterators.
The `Order`s are stored in a `Slab`, in contiguous memory, which allows for fast lookup and quick iteration. 
A lookup table is created for each indexed field, which maps the index key to a index in the `Slab`.
The exact type used for these depends on the annotations.
For `hashed_unique` and `hashed_non_unique` a `HashMap` is used, for `ordered_unique` and `ordered_non_unique` a `BTreeMap` is used.
* When inserting an element, we add it to the backing store, then add elements to each lookup table pointing to the index in the backing store.
* When retrieving elements for a given key, we lookup the key in the lookup table, then retrieve the item at that index in the backing store.
* When removing an element for a given key, we do the same, but we then must also remove keys from all the other lookup tables before returning the element.
* When iterating over an index, we use the default iterators for the lookup table, then simply retrieve the element at the given index in the backing store.
* When updating un-indexed fields, we lookup the element(s) through the given key, then apply the closure to modify just the unindexed fields in-place.
We then return a reference to the modified element(s).
If the key doesn't match, the closure won't be applied, and Option::None will be returned.
* When modifying indexed fields of an element, we do the same process, but the closure takes a mutable reference to the whole element.
Any fields, indexed and un-indexed can be modified.
We must then update all the lookup tables to account for any changes to indexed fields, so this is slower than an un-indexed update.


```rust
struct MultiIndexOrderMap {
    _store: slab::Slab<Order>,
    _order_id_index: HashMap<u32, usize, rustc_hash::FxBuildHasher>,
    _timestamp_index: BTreeMap<u64, usize>,
    _trader_name_index: HashMap<String, BTreeSet<usize>, rustc_hash::FxBuildHasher>,
}

struct MultiIndexOrderMapOrderIdIter<'a> {
    ...
}

struct MultiIndexOrderMapTimestampIter<'a> {
    ...
}

struct MultiIndexOrderMapTraderNameIter<'a> {
    ...
}

struct OrderMutIter<'a> {
    ...
}

impl MultiIndexOrderMap {
    fn try_insert(&mut self, elem: Order) -> Result<&Order, MultiIndexMapError<Order>>;
    fn insert(&mut self, elem: Order) -> &Order;
    
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn clear(&mut self);
    
    fn get_by_order_id(&self, key: &u32) -> Option<&Order>;
    fn get_by_timestamp(&self, key: &u64) -> Option<&Order>;
    fn get_by_trader_name(&self, key: &String) -> Vec<&Order>;

    fn get_mut_by_order_id(&mut self, key: &u32) -> Option<(&mut bool, &mut u64)>;
    fn get_mut_by_timestamp(&mut self, key: &u64) -> Option<(&mut bool, &mut u64)>;
    fn get_mut_by_trader_name(&mut self, key: &String) -> Vec<(&mut bool, &mut u64)>;

    fn update_by_order_id(&mut self, key: &u32, f: impl FnOnce(&mut bool, &mut u64)) -> Option<&Order>;
    fn update_by_timestamp(&mut self, key: &u64, f: impl FnOnce(&mut bool, &mut u64)) -> Option<&Order>;
    fn update_by_trader_name(&mut self, key: &String, f: impl FnMut(&mut bool, &mut u64)) -> Vec<&Order>;
    
    fn modify_by_order_id(&mut self, key: &u32, f: impl FnOnce(&mut Order)) -> Option<&Order>;
    fn modify_by_timestamp(&mut self, key: &u64, f: impl FnOnce(&mut Order)) -> Option<&Order>;
    fn modify_by_trader_name(&mut self, key: &String, f: impl FnMut(&mut Order)) -> Vec<&Order>;
    
    fn remove_by_order_id(&mut self, key: &u32) -> Option<Order>;
    fn remove_by_timestamp(&mut self, key: &u64) -> Option<Order>;
    fn remove_by_trader_name(&mut self, key: &String) -> Vec<Order>;
    
    fn iter(&self) -> slab::Iter<Order>;
    fn iter_mut(&mut self) -> OrderMutIter;
    
    fn iter_by_order_id(&self) -> MultiIndexOrderMapOrderIdIter;
    fn iter_by_timestamp(&self) -> MultiIndexOrderMapTimestampIter;
    fn iter_by_trader_name(&self) -> MultiIndexOrderMapTraderNameIter;
}

impl<'a> Iterator for OrderMutIter<'a> {
    type Item = (&mut bool, &mut u64);

    fn next(&mut self) -> Option<Self::Item> {
        ...
    }
}
```

# Dependencies
See [Cargo.toml](Cargo.toml) for information on each dependency.

# Future work
* Potentially a vector-map style lookup table would be very quick for small tables with integer indexes.
* Allow overwriting behaviour upon inserting a duplicate unique index, returning a Vec of the overwritten elements.
* Implement [clever tricks](https://www.boost.org/doc/libs/1_36_0/libs/multi_index/doc/performance.html) used in boost::multi_index_containers to improve performance.

