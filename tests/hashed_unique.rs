use multi_index_map::MultiIndexMap;
use multi_index_test_element::MultiIndexTestElementMap;

#[derive(Hash, PartialEq, Eq, Clone)]
struct TestNonPrimitiveType(u64);

#[derive(MultiIndexMap, Clone)]
struct TestElement {
    #[multi_index(hashed_unique)]
    field1: TestNonPrimitiveType,
    field2: String,
}

#[test]
fn test_insert_and_get() {
    let mut map = MultiIndexTestElementMap::default();
    let elem1 = TestElement {
        field1: TestNonPrimitiveType(42),
        field2: "ElementOne".to_string(),
    };
    map.insert(elem1);

    let elem1_ref = map.get_by_field1(&TestNonPrimitiveType(42)).unwrap();
    assert_eq!(elem1_ref.field1.0, 42);
    assert_eq!(elem1_ref.field2, "ElementOne");
    assert_eq!(map.len(), 1);
}

#[test]
fn test_insert_and_remove_by_field1() {
    let mut map = MultiIndexTestElementMap::default();
    let elem1 = TestElement {
        field1: TestNonPrimitiveType(42),
        field2: "ElementOne".to_string(),
    };
    let elem2 = TestElement {
        field1: TestNonPrimitiveType(43),
        field2: "ElementTwo".to_string(),
    };
    map.insert(elem1);
    map.insert(elem2);

    let elem1 = map.remove_by_field1(&TestNonPrimitiveType(42)).unwrap();
    assert_eq!(elem1.field1.0, 42);
    assert_eq!(elem1.field2, "ElementOne");
    assert_eq!(map.len(), 1);

    let elem2 = map.remove_by_field1(&TestNonPrimitiveType(43)).unwrap();
    assert_eq!(elem2.field1.0, 43);
    assert_eq!(elem2.field2, "ElementTwo");
    assert!(map.is_empty());
}

#[test]
fn test_unsafe_mutate_by_field1() {
    let mut map = MultiIndexTestElementMap::default();
    let elem1 = TestElement {
        field1: TestNonPrimitiveType(42),
        field2: "ElementOne".to_string(),
    };
    map.insert(elem1);

    let elem1_ref = unsafe { map.get_mut_by_field1(&TestNonPrimitiveType(42)).unwrap() };
    elem1_ref.field2 = "ModifiedElementOne".to_string();

    let elem1_ref = map.get_by_field1(&TestNonPrimitiveType(42)).unwrap();
    assert_eq!(elem1_ref.field1.0, 42);
    assert_eq!(elem1_ref.field2, "ModifiedElementOne");
    assert_eq!(map.len(), 1);
}

#[test]
fn test_modify_by_field1() {
    let mut map = MultiIndexTestElementMap::default();
    let elem1 = TestElement {
        field1: TestNonPrimitiveType(42),
        field2: "ElementOne".to_string(),
    };
    let elem2 = TestElement {
        field1: TestNonPrimitiveType(43),
        field2: "ElementTwo".to_string(),
    };
    map.insert(elem1);
    map.insert(elem2);

    let elem1_ref = map
        .modify_by_field1(&TestNonPrimitiveType(42), |test_elem| {
            test_elem.field1 = TestNonPrimitiveType(44)
        })
        .unwrap();

    assert_eq!(elem1_ref.field1.0, 44);
    assert_eq!(elem1_ref.field2, "ElementOne");
}

#[test]
fn test_insert_violate_uniqueness() {
    let mut map = MultiIndexTestElementMap::default();
    let elem1 = TestElement {
        field1: TestNonPrimitiveType(42),
        field2: "ElementOne".into(),
    };
    map.insert(elem1);

    let elem2 = TestElement {
        field1: TestNonPrimitiveType(42),
        field2: "ElementTwo".into(),
    };

    let res = std::panic::catch_unwind(move || {
        map.insert(elem2);
    });

    res.expect_err("Expected to violate uniqueness constraint");
}

#[test]
fn test_modify_violate_uniqueness() {
    let mut map = MultiIndexTestElementMap::default();
    let elem1 = TestElement {
        field1: TestNonPrimitiveType(42),
        field2: "ElementOne".into(),
    };
    map.insert(elem1);

    let elem2 = TestElement {
        field1: TestNonPrimitiveType(43),
        field2: "ElementTwo".into(),
    };

    map.insert(elem2);

    let res = std::panic::catch_unwind(move || {
        map.modify_by_field1(&TestNonPrimitiveType(43), |e| {
            e.field1 = TestNonPrimitiveType(42)
        });
    });

    res.expect_err("Expected to violate uniqueness constraint");
}
