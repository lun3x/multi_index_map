# Boost-Inspired Prototype

This example is a manually expanded design reference for the experimental `MultiIndexMap2` derive.
It does not change the existing `MultiIndexMap` derive.

Run it with:

```text
cargo run --example boost_inspired
cargo test --example boost_inspired
```

## File Roles

- `multi_index_map::__private` contains the safe, integer-linked hashed and ordered index engines
  shared with generated maps.
- `multi_index_map::views` defines the public capability traits shared by generated index views.
- `order_map.rs` represents code that a future proc macro could generate for one concrete element
  type.
- `main.rs` demonstrates the intended typed-view API and contains correctness tests.

## Architecture

`OrderMap` owns one `Slab<OrderNode>`. Each `OrderNode` stores one `Order` and one link record for
each index:

- hashed unique `id`
- ordered unique `timestamp`
- hashed non-unique `trader`
- ordered non-unique `price`

The indexes contain only bucket roots, tree roots, counts, and configuration. Indexed field values
are neither cloned nor separately allocated. A known node can be removed from every index directly
through its embedded links.

Hashed indices use cached hashes and doubly linked bucket chains. Equivalent values in non-unique
hashed indices remain contiguous. Ordered indices use red-black trees with embedded parent, child,
and color fields.

Immutable iteration follows the embedded links without allocating. Batch mutation deliberately
snapshots the original matching `NodeId`s so each original match is processed exactly once.

## Generic Index Selection

`OrderMap` selects an index through its marker type rather than generating a different accessor
method for every indexed field:

```rust
orders.by::<ByTimestamp>().range(start..end);
orders.by_mut::<ByTrader>().remove_all("John");
```

The example-local `OrderMapIndex` trait uses generic associated types to map each marker to its
immutable and mutable named view types. Its constructor methods let the two map accessors construct
the selected views while preserving the borrow lifetime. `ById`, `ByTimestamp`, `ByTrader`, and
`ByPrice` also remain the corresponding internal hash/tree index specs.

## View Capabilities

Views implement small, composable traits that describe the operations their index category
supports:

- every view implements `IndexView`
- unique views implement `UniqueView`
- non-unique views implement `NonUniqueView`
- ordered views additionally implement `OrderedView`
- mutable views implement the corresponding read traits plus `UniqueViewMut` or
  `NonUniqueViewMut`

The generated inherent methods remain the primary ergonomic API and support richer borrowed-query
types. Capability-trait methods use the exact associated key type, making them suitable for generic
algorithms without forcing hashed and ordered borrowed-query bounds into one trait.

Because public traits expose iterator associated types, `order_map.rs` defines thin, named iterator
wrappers for each generated traversal type. These wrappers hide private node and index-spec types
without boxing or allocation.

## Compatibility Facade

`OrderMap` also demonstrates the deprecated field-named methods generated maps can provide during
migration:

```text
get_by_*
get_mut_by_*
modify_by_*
update_by_*
remove_by_*
iter_by_*
```

These methods preserve the existing closure shapes and `Option`/`Vec` return types while delegating
to the new index and coordinated-mutation machinery. Each method is deprecated with its view-based
replacement. `get_by_*` and `update_by_*` continue to support borrowed queries, including both
`&String` and `&str` for the `trader` field.

Legacy non-unique methods allocate their returned `Vec`; its ordering is not part of the
compatibility contract. `get_mut_by_*` safely exposes only the unindexed `note` and `filled` fields
by sorting snapshotted node IDs and walking the slab mutably once.

The new conflict semantics remain authoritative. A compatibility `modify_by_*` method panics when a
uniqueness conflict occurs, because its legacy signature cannot return `Conflict`, but conflicting
elements are removed and all indices are repaired before the panic. Non-unique compatibility
modifiers finish processing the original snapshotted batch before reporting a conflict.

## Mutation Semantics

- `replace` is atomic. Every unique constraint is checked before the stored value changes.
- `modify` is clone-free. Each index keeps the node in place when its ordering remains valid and
  otherwise directly unlinks and reinserts it.
- A uniqueness conflict during `modify` removes and returns the modified value.
- A panic inside a modifier removes the partially modified value before resuming the panic.
- `update` exposes only unindexed fields through `OrderUpdate`, so it performs no index work.

## Deliberate Deferrals

The prototype and initial derive do not include persistent handles, projection, packed red-black
color bits, Boost's specialized hashed group-skip encoding, custom comparators, serde, or capacity
APIs.
