Version 0.15.0 (2025-05-21)
==========================

- Modify get_mut_by_ methods such that they return only the unindexed fields of the element, allowing full mutable access without risk of breaking the index.

Version 0.14.0 (2025-05-20)
==========================

- Allow hasher to be configured for each MultiIndexMap through the `#[multi_index_hash]` attribute.

Version 0.13.0 (2025-05-09)
==========================

- Allow generics in both indexed and unindexed fields. Unindexed fields can also be references with lifetimes. Generic fields must implement the traits Hash+Eq or Ord+Eq. Indexed generic fields must additionally implement Clone.

Version 0.12.1 (2025-04-14)
==========================

- Bump rustc-hash dependency to 2.1 to use newer, faster hash function

Version 0.12.0 (2025-04-14)
==========================

- Republish 0.11.1 as 0.12.0 due to SemVer violation caused by accidental inclusion of new Borrowed key type feature.

Version 0.11.1 (2025-04-14)
==========================

- Swap proc-macro-error dependency for proc-macro-error2, as the former has been abandoned.
- Update convert-case dependency to 0.8
- Allow callers to use Borrowed key types to access fields, eg. &str can be used rather than &String

Version 0.11.0 (2023-11-02)
==========================

- Add `try_insert` method to allow fallible insertion and return shared reference to newly inserted element.
- Change `insert` method to return shared reference to newly inserted element.

Version 0.10.0 (2023-11-02)
==========================

- Allow optional `#[multi_index_derive()]` attribute on element, to enable deriving of traits on the generated MultiIndexMap. eg. `#[multi_index_derive(Clone, Debug)]`.
- Add feature `serde` to allow deriving of `Serialize` and `Deserialize` on MultiIndexMap.
- Remove feature `trivial_bounds`, the same result can be acheived on stable rust with the `multi_index_derive` attribute.

Version 0.9.0 (2023-09-07)
==========================

- Add `trivial_bounds` feature to automatically derive Debug and Clone impls if all elements support Debug and Clone. This feature requires nightly rust, because it uses the `trivial_bounds` nightly feature.

Version 0.8.1 (2023-08-30)
==========================

- Allow FnMut closures in `modify_by_` methods.

Version 0.8.0 (2023-08-30)
==========================

- Remove `Clone` requirement on elements, now only the indexed fields must implement Clone. This should be helpful when storing non-Clonable types in un-indexed fields.
- If the MultiIndexMap does need to be Cloned, this must be implemented manually, however this should be fairly simple to do next to where the element is defined. See `examples/main.rs`.

Version 0.7.1 (2023-08-30)
==========================

- Refactor and cleanup lots of code, also further reduce work done at compile time, by only generating identifiers for each field once.
- Implement work necessary to remove Clone requirement, however this will be fully removed in the next release.

Version 0.7.0 (2023-08-29)
==========================

- Add `update_by_` methods and deprecate `get_mut_by_` methods. The new methods are equivalently useful, but safe and equally performant.

Version 0.6.2 (2023-08-15)
==========================

- Reduce work done at compile time by only looking up ordering and uniqueness once per field.
- Improve error messages, so all invalid attributes will be highlighted.
- Use version 2 resolver in Cargo.toml

Version 0.6.1 (2023-08-10)
==========================

- Merge @wyjin PR to implement the following changes:
    - fix issue #27 to support other Derive attributes in any order
- Refactor codebase for a large clean up

Version 0.6.0 (2023-06-23)
==========================

- Merge @wyjin PR to implement the following changes:
    - add `modify_by_` and `get_mut_by_` for non-unique indexes
    - use BTreeSet to store equivalent elements in a non-unique index to improve insert/remove/modify performance
    - add capacity-adjustment methods `shrink_to_fit`, `reserve`, and `with_capacity`
    - bug fix when modifying non_unique indexes that caused only a single element to be modified
    - add benchmarks
- Remove requirement for slab and rustc_hash in dependee's Cargo.toml by restucturing package, splitting it into multi_index_map_derive and multi_index_map

Version 0.5.0 (2023-04-24)
==========================

- Set MultiIndexMap to same visibility as provided Element. Set each field's relevant methods to the visibility of that field. This allows finer-grained control of method visibility/privacy.
- Remove inner `multi_index_<element_name>` module. Previously this was used to avoid polluting the outer namespace with the Iterators for each field, however now users can now control the visibility per-field, so can create their own inner module if necessary to avoid polluting namespace.
- Change `iter_by_` methods. Now they take `&self`, previously they required `&mut self` but this is not necessary.

Version 0.4.2 (2022-09-06)
==========================

- Add `clear()` method to clear the backing storage and all indexes.

Version 0.4.1 (2022-09-02)
==========================

- Prevent uniqueness constraints being violated by panicking upon any `insert` or `modify` that would result in violation. Previously to this version violations would result in overwriting the indexes to point to the new element, but the old element would remain in the backing storage, accessible only through the general `iter()` / `iter_mut()` methods, and visible in the `is_empty()` and `len()` methods.

Version 0.4.0 (2022-08-26)
==========================

- Fix bug with multiple non-unique indexes, whereby removal from one non-unique index could cause elements to become inaccessible through other non-unique indexes.
- Rename `multi_index` namespace to `multi_index_<element_name>` to avoid clashes when defining multiple MultiIndexMaps in a single namespace.

Version 0.3.0 (2022-08-04)
==========================

- Implement `ordered_non_unique` and provide `get_mut_by_` accessors for both `non_unique` indexes.
- Clean up `IndexKind` enum to orthogonally represent Uniqueness and Ordering.

Version 0.2.1 (2022-07-14)
==========================

- Remove requirement for all field indexes to implement `Copy`.
- Derive `Clone` on the resulting map, in order to give better error messages that all fields need to implement `Clone`.

Version 0.2.0 (2022-07-14)
==========================

- Add `hashed_non_unique` field attribute, with associated `insert_by_` and `iter_by_` accessors.
- Add initial test for `hashed_non_unique`.
- Ensure non-primitive types (ie. user-defined structs) are imported to the `multi_index` module to be used as indexes.