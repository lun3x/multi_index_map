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