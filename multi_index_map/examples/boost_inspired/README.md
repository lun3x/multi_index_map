# Boost-Inspired Prototype

This example is a manually expanded design target for a future proc-macro implementation. It does
not change the existing `MultiIndexMap` derive.

Run it with:

```text
cargo run --example boost_inspired
cargo test --example boost_inspired
```

## File Roles

- `index.rs` represents reusable library machinery. It contains safe, integer-linked hashed and
  ordered index engines.
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

## Mutation Semantics

- `replace` is atomic. Every unique constraint is checked before the stored value changes.
- `modify` is clone-free. Each index keeps the node in place when its ordering remains valid and
  otherwise directly unlinks and reinserts it.
- A uniqueness conflict during `modify` removes and returns the modified value.
- A panic inside a modifier removes the partially modified value before resuming the panic.
- `update` exposes only unindexed fields through `OrderUpdate`, so it performs no index work.

## Deliberate Deferrals

The prototype does not include proc-macro generation, persistent handles, projection, packed
red-black color bits, Boost's specialized hashed group-skip encoding, custom comparators, serde,
capacity APIs, or compatibility wrappers for the current field-named API.
