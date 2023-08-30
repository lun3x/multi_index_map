#![cfg_attr(feature = "trivial_bounds", feature(trivial_bounds))]

use multi_index_map::MultiIndexMap;

#[derive(MultiIndexMap)]
struct MultipleOrderedNonUniqueStruct {
    #[multi_index(ordered_non_unique)]
    field1: u32,
    #[multi_index(ordered_non_unique)]
    field2: u64,
}

#[test]
fn test_remove_ordered_non_unique_field1_get_ordered_non_unique_field2() {
    let mut map = MultiIndexMultipleOrderedNonUniqueStructMap::default();

    map.insert(MultipleOrderedNonUniqueStruct {
        field1: 1,
        field2: 999,
    });
    map.insert(MultipleOrderedNonUniqueStruct {
        field1: 2,
        field2: 999,
    });

    let a = map.remove_by_field1(&1);
    let b = map.get_by_field2(&999);

    assert_eq!(a.len(), 1);
    assert_eq!(b.len(), 1);
}

#[test]
fn test_remove_ordered_non_unique_field2_get_ordered_non_unique_field1() {
    let mut map = MultiIndexMultipleOrderedNonUniqueStructMap::default();

    map.insert(MultipleOrderedNonUniqueStruct {
        field1: 1,
        field2: 999,
    });
    map.insert(MultipleOrderedNonUniqueStruct {
        field1: 2,
        field2: 999,
    });

    let a = map.remove_by_field2(&999);
    let b = map.get_by_field1(&1);
    let c = map.get_by_field1(&2);

    assert_eq!(a.len(), 2);
    assert_eq!(b.len(), 0);
    assert_eq!(c.len(), 0);
}

#[derive(MultiIndexMap, Clone)]
struct OrderedNonUniqueAndHashedNonUniqueStruct {
    #[multi_index(hashed_non_unique)]
    field1: u32,
    #[multi_index(ordered_non_unique)]
    field2: u64,
}

#[test]
fn test_remove_hashed_non_unique_field1_get_ordered_non_unique_field2() {
    let mut map = MultiIndexOrderedNonUniqueAndHashedNonUniqueStructMap::default();

    map.insert(OrderedNonUniqueAndHashedNonUniqueStruct {
        field1: 1,
        field2: 999,
    });
    map.insert(OrderedNonUniqueAndHashedNonUniqueStruct {
        field1: 2,
        field2: 999,
    });

    let a = map.remove_by_field1(&1);
    let b = map.get_by_field2(&999);

    assert_eq!(a.len(), 1);
    assert_eq!(b.len(), 1);
}

#[test]
fn test_remove_ordered_non_unique_field2_get_hashed_non_unique_field1() {
    let mut map = MultiIndexOrderedNonUniqueAndHashedNonUniqueStructMap::default();

    map.insert(OrderedNonUniqueAndHashedNonUniqueStruct {
        field1: 1,
        field2: 999,
    });
    map.insert(OrderedNonUniqueAndHashedNonUniqueStruct {
        field1: 2,
        field2: 999,
    });

    let a = map.remove_by_field2(&999);
    let b = map.get_by_field1(&1);
    let c = map.get_by_field1(&2);

    assert_eq!(a.len(), 2);
    assert_eq!(b.len(), 0);
    assert_eq!(c.len(), 0);
}

#[derive(MultiIndexMap, Clone)]
struct MultipleHashedNonUniqueStruct {
    #[multi_index(hashed_non_unique)]
    field1: u32,
    #[multi_index(ordered_non_unique)]
    field2: u64,
}

#[test]
fn test_remove_hashed_non_unique_field1_get_hashed_non_unique_field2() {
    let mut map = MultiIndexMultipleHashedNonUniqueStructMap::default();

    map.insert(MultipleHashedNonUniqueStruct {
        field1: 1,
        field2: 999,
    });
    map.insert(MultipleHashedNonUniqueStruct {
        field1: 2,
        field2: 999,
    });

    let a = map.remove_by_field1(&1);
    let b = map.get_by_field2(&999);

    assert_eq!(a.len(), 1);
    assert_eq!(b.len(), 1);
}

#[test]
fn test_remove_hashed_non_unique_field2_get_hashed_non_unique_field1() {
    let mut map = MultiIndexMultipleHashedNonUniqueStructMap::default();

    map.insert(MultipleHashedNonUniqueStruct {
        field1: 1,
        field2: 999,
    });
    map.insert(MultipleHashedNonUniqueStruct {
        field1: 2,
        field2: 999,
    });

    let a = map.remove_by_field2(&999);
    let b = map.get_by_field1(&1);
    let c = map.get_by_field1(&2);

    assert_eq!(a.len(), 2);
    assert_eq!(b.len(), 0);
    assert_eq!(c.len(), 0);
}

#[test]
fn test_clear() {
    let mut map = MultiIndexMultipleOrderedNonUniqueStructMap::default();

    map.insert(MultipleOrderedNonUniqueStruct {
        field1: 1,
        field2: 999,
    });
    map.insert(MultipleOrderedNonUniqueStruct {
        field1: 2,
        field2: 999,
    });
    assert_eq!(map.len(), 2);

    map.clear();
    assert!(map.is_empty());

    let a = map.remove_by_field2(&999);
    let b = map.remove_by_field1(&1);
    let c = map.remove_by_field1(&2);
    assert!(a.is_empty());
    assert!(b.is_empty());
    assert!(c.is_empty());
}
