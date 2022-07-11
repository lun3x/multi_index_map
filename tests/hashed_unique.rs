use multi_index::MultiIndexTestElementMap;
use multi_index_map::MultiIndexMap;

#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
struct TestNonPrimitiveType(u64);

#[derive(MultiIndexMap, Clone)]
struct TestElement {
    #[multi_index(hashed_unique)]
    field1: u32,
    #[multi_index(hashed_unique)]
    field2: TestNonPrimitiveType,
    field3: String,
}

#[test]
fn test_insert_and_get() {
    let mut map = MultiIndexTestElementMap::default();
    let elem1 = TestElement {
        field1: 1,
        field2: TestNonPrimitiveType(42),
        field3: "ElementOneFieldThree".to_string(),
    };
    map.insert(elem1);
    let elem1_ref = map.get_by_field1(&1).unwrap();
    assert_eq!(elem1_ref.field1, 1);
    assert_eq!(elem1_ref.field2, TestNonPrimitiveType(42));
    assert_eq!(elem1_ref.field3, "ElementOneFieldThree".to_string());
    assert_eq!(map.len(), 1);

    let elem1_ref = map.get_by_field2(&TestNonPrimitiveType(42)).unwrap();
    assert_eq!(elem1_ref.field1, 1);
    assert_eq!(elem1_ref.field2, TestNonPrimitiveType(42));
    assert_eq!(elem1_ref.field3, "ElementOneFieldThree".to_string());
    assert_eq!(map.len(), 1);
}

#[test]
fn test_insert_and_remove_by_field1() {
    let mut map = MultiIndexTestElementMap::default();
    let elem1 = TestElement {
        field1: 1,
        field2: TestNonPrimitiveType(42),
        field3: "ElementOneFieldThree".to_string(),
    };
    let elem2 = TestElement {
        field1: 2,
        field2: TestNonPrimitiveType(43),
        field3: "ElementTwoFieldThree".to_string(),
    };
    map.insert(elem1);
    map.insert(elem2);

    let elem1 = map.remove_by_field1(&1).unwrap();
    assert_eq!(elem1.field1, 1);
    assert_eq!(elem1.field2, TestNonPrimitiveType(42));
    assert_eq!(elem1.field3, "ElementOneFieldThree".to_string());
    assert_eq!(map.len(), 1);

    let elem2 = map.remove_by_field1(&2).unwrap();
    assert_eq!(elem2.field1, 2);
    assert_eq!(elem2.field2, TestNonPrimitiveType(43));
    assert_eq!(elem2.field3, "ElementTwoFieldThree".to_string());
    assert!(map.is_empty());
}

#[test]
fn test_insert_and_remove_by_field2() {
    let mut map = MultiIndexTestElementMap::default();
    let elem1 = TestElement {
        field1: 1,
        field2: TestNonPrimitiveType(42),
        field3: "ElementOneFieldThree".to_string(),
    };
    let elem2 = TestElement {
        field1: 2,
        field2: TestNonPrimitiveType(43),
        field3: "ElementTwoFieldThree".to_string(),
    };
    map.insert(elem1);
    map.insert(elem2);

    let elem1 = map.remove_by_field2(&TestNonPrimitiveType(42)).unwrap();
    assert_eq!(elem1.field1, 1);
    assert_eq!(elem1.field2, TestNonPrimitiveType(42));
    assert_eq!(elem1.field3, "ElementOneFieldThree".to_string());
    assert_eq!(map.len(), 1);

    let elem2 = map.remove_by_field2(&TestNonPrimitiveType(43)).unwrap();
    assert_eq!(elem2.field1, 2);
    assert_eq!(elem2.field2, TestNonPrimitiveType(43));
    assert_eq!(elem2.field3, "ElementTwoFieldThree".to_string());
    assert!(map.is_empty());
}
