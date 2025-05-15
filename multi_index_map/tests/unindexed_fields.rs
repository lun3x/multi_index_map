use multi_index_map::MultiIndexMap;
use serde_json::map::Iter;

#[derive(Hash, PartialEq, Eq, Clone)]
struct TestCloneableType(u64);

#[derive(Hash, PartialEq, Eq)]
struct TestNonCloneableType(u64);

#[derive(MultiIndexMap, PartialEq)]
struct TestNonCloneableElement {
    #[multi_index(hashed_unique)]
    field1: TestCloneableType,
    field2: TestNonCloneableType,
}

#[test]
fn test_unindexed_fields_dont_need_to_derive_clone() {
    let mut map = MultiIndexTestNonCloneableElementMap::default();

    let elem1 = TestNonCloneableElement {
        field1: TestCloneableType(42),
        field2: TestNonCloneableType(1),
    };
    map.insert(elem1);
}
