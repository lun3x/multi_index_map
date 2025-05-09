use multi_index_map::MultiIndexMap;

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
struct TestNonPrimitiveType(u64);

#[derive(MultiIndexMap, Clone, Debug)]
#[multi_index_derive(Clone, Debug)]
struct TestElement<T> {
    #[multi_index(hashed_unique)]
    field1: TestNonPrimitiveType,
    #[allow(dead_code)]
    field2: T,
}

#[test]
fn should_compile() {
    let mut map = MultiIndexTestElementMap::default();

    // check that formatting produces non empty strings
    assert!(!format!("{:?}", map._field1_index).is_empty());
    assert!(!format!("{:?}", map._store).is_empty());
    assert!(!format!("{:?}", map).is_empty());

    // T is resolved to String
    let elem1: TestElement<String> = TestElement {
        field1: TestNonPrimitiveType(42),
        field2: "ElementOne".to_string(),
    };
    map.insert(elem1);

    let msg = format!("{:?}", map);
    // check if indexed fields are present in debug output
    assert!(msg.contains("42"));

    let new_map = map.clone();
    assert_eq!(new_map.len(), map.len());
}
