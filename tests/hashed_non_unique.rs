use multi_index::MultiIndexTestElementMap;
use multi_index_map::MultiIndexMap;

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
struct TestNonPrimitiveType(u64);

#[derive(MultiIndexMap, Clone, Debug)]
struct TestElement {
    #[multi_index(hashed_non_unique)]
    field1: TestNonPrimitiveType,
    field2: String,
}

#[test]
fn test_iter_by_field1() {
    let mut map = MultiIndexTestElementMap::default();
    let elem1 = TestElement {
        field1: TestNonPrimitiveType(42),
        field2: "ElementOne".to_string(),
    };
    let elem2 = TestElement {
        field1: TestNonPrimitiveType(42),
        field2: "ElementTwo".to_string(),
    };
    map.insert(elem2);
    map.insert(elem1);

    for elem in map.iter_by_field1() {
        println!("{}", elem.field2);
    }

    let elems = map.remove_by_field1(&TestNonPrimitiveType(42));
    println!("{elems:?}");
}
