#![allow(clippy::duplicated_attributes)]
#![allow(deprecated)]

use multi_index_map::{MultiIndexMap, MultiIndexSelector};
use std::fmt::Debug;
use std::hash::Hash;
use std::panic::{catch_unwind, AssertUnwindSafe};

#[derive(MultiIndexSelector)]
#[multi_index(hashed_unique)]
struct ByDerivedId;

#[derive(Clone, Debug, Eq, MultiIndexMap, PartialEq)]
#[multi_index_derive(Clone, Debug)]
struct DerivedRecord {
    #[multi_index(by(ByDerivedId))]
    id: u64,
    #[multi_index(hashed_non_unique)]
    group: String,
    value: Vec<u64>,
}

#[derive(Clone)]
struct CloneOnlyPayload(u64);

#[derive(Clone, MultiIndexMap)]
#[multi_index_derive(Clone)]
struct CloneOnlyRecord {
    #[multi_index(hashed_unique)]
    id: u64,
    payload: CloneOnlyPayload,
}

#[derive(Debug)]
#[allow(dead_code)]
struct DebugOnlyPayload(u64);

#[derive(Debug, MultiIndexMap)]
#[multi_index_derive(Debug, Default)]
#[multi_index_derive(Debug, Default)]
#[allow(dead_code)]
struct DebugOnlyRecord {
    #[multi_index(ordered_unique)]
    id: u64,
    payload: DebugOnlyPayload,
}

#[derive(MultiIndexSelector)]
#[multi_index(hashed_unique)]
struct ByGenericDerivedKey;

#[derive(Clone, Debug, MultiIndexMap)]
#[multi_index_derive(Clone)]
#[multi_index_derive(Debug, Default, Clone)]
struct GenericDerived<'a, K: Clone + Debug + Eq + Hash, T: Clone + Debug, const N: usize> {
    #[multi_index(by(ByGenericDerivedKey))]
    key: K,
    payload: T,
    borrowed: &'a str,
    bytes: [u8; N],
}

#[derive(Debug, Eq, Hash, PartialEq)]
struct CollapsingKey(u64);

impl Clone for CollapsingKey {
    fn clone(&self) -> Self {
        Self(0)
    }
}

#[derive(Clone, MultiIndexMap)]
#[multi_index_derive(Clone)]
struct CollapsingRecord {
    #[multi_index(hashed_unique)]
    key: CollapsingKey,
}

fn assert_clone<T: Clone>() {}
fn assert_debug<T: std::fmt::Debug>() {}

#[test]
#[allow(deprecated)]
fn clone_rebuilds_hybrid_indexes_after_holes_and_is_independent() {
    assert_clone::<MultiIndexDerivedRecordMap>();
    assert_debug::<MultiIndexDerivedRecordMap>();

    let mut original = MultiIndexDerivedRecordMap::new();
    for id in 1..=4 {
        original.insert(DerivedRecord {
            id,
            group: if id % 2 == 0 { "even" } else { "odd" }.to_owned(),
            value: vec![id],
        });
    }
    original.by_mut::<ByDerivedId>().remove(&2);
    original.insert(DerivedRecord {
        id: 5,
        group: "odd".to_owned(),
        value: vec![5],
    });

    let mut cloned = original.clone();
    original.validate().unwrap();
    cloned.validate().unwrap();
    assert_eq!(cloned.len(), original.len());
    assert_eq!(cloned.by::<ByDerivedId>().get(&1).unwrap().value, vec![1]);
    assert_eq!(cloned.get_by_group("odd").len(), 3);

    cloned
        .by_mut::<ByDerivedId>()
        .modify(&1, |record| record.group = "changed".to_owned())
        .unwrap();
    assert_eq!(cloned.get_by_group("changed").len(), 1);
    assert_eq!(original.get_by_group("changed").len(), 0);
    original.validate().unwrap();
    cloned.validate().unwrap();
}

#[test]
fn debug_formats_values_without_private_representation() {
    let mut map = MultiIndexDerivedRecordMap::new();
    map.insert(DerivedRecord {
        id: 7,
        group: "group".to_owned(),
        value: vec![8, 9],
    });

    let output = format!("{map:?}");
    assert!(output.starts_with("MultiIndexDerivedRecordMap"));
    assert!(output.contains("values"));
    assert!(output.contains("DerivedRecord"));
    assert!(output.contains("group"));
    for private_name in ["inner", "__mim", "HashLink", "OrderedLink", "nodes"] {
        assert!(!output.contains(private_name));
    }
}

#[test]
#[allow(deprecated)]
fn clone_and_debug_are_independent_and_default_is_a_noop() {
    assert_clone::<MultiIndexCloneOnlyRecordMap>();
    let mut clone_only = MultiIndexCloneOnlyRecordMap::new();
    clone_only.insert(CloneOnlyRecord {
        id: 1,
        payload: CloneOnlyPayload(9),
    });
    let cloned = clone_only.clone();
    assert_eq!(cloned.get_by_id(&1).unwrap().payload.0, 9);

    assert_debug::<MultiIndexDebugOnlyRecordMap>();
    let mut debug_only: MultiIndexDebugOnlyRecordMap = Default::default();
    debug_only.insert(DebugOnlyRecord {
        id: 2,
        payload: DebugOnlyPayload(10),
    });
    let output = format!("{debug_only:?}");
    assert!(output.contains("DebugOnlyPayload(10)"));
}

#[test]
fn generated_traits_support_lifetime_type_and_const_generics() {
    let borrowed = String::from("borrowed");
    let mut original = MultiIndexGenericDerivedMap::new();
    original.insert(GenericDerived {
        key: String::from("key"),
        payload: vec![1_u64],
        borrowed: &borrowed,
        bytes: [2_u8; 3],
    });

    let cloned = original.clone();
    assert_eq!(
        cloned
            .by::<ByGenericDerivedKey>()
            .get("key")
            .unwrap()
            .borrowed,
        "borrowed"
    );
    assert!(format!("{cloned:?}").contains("bytes: [2, 2, 2]"));
    cloned.validate().unwrap();
}

#[test]
fn clone_panics_if_element_clone_breaks_a_unique_index() {
    let mut map = MultiIndexCollapsingRecordMap::new();
    map.insert(CollapsingRecord {
        key: CollapsingKey(1),
    });
    map.insert(CollapsingRecord {
        key: CollapsingKey(2),
    });

    let panic = catch_unwind(AssertUnwindSafe(|| map.clone()));
    assert!(panic.is_err());
    map.validate().unwrap();
}
