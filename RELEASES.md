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