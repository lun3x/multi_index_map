use multi_index_map::MultiIndexMap;

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
struct TestNonPrimitiveType1(u64);

#[derive(PartialOrd, Ord, PartialEq, Eq, Clone, Debug)]
struct TestNonPrimitiveType2(u64);

#[derive(MultiIndexMap, Clone, Debug)]
#[multi_index_derive(Clone, Debug)]
struct TestElement<F1: Clone + Eq + std::hash::Hash, F2: Clone + Eq + Ord, T, Q> {
    #[multi_index(hashed_unique)]
    field1: F1,
    #[multi_index(ordered_unique)]
    field2: F2,
    #[multi_index(hashed_non_unique)]
    field3: F1,
    #[multi_index(ordered_non_unique)]
    field4: F2,
    #[allow(dead_code)]
    field5: T,
    #[allow(dead_code)]
    field6: Q,
}

#[test]
fn should_compile() {
    let mut map = MultiIndexTestElementMap::default();

    let elem1 = TestElement {
        field1: TestNonPrimitiveType1(42),
        field2: TestNonPrimitiveType2(99),
        field3: TestNonPrimitiveType1(41),
        field4: TestNonPrimitiveType2(98),
        field5: "ElementOne".to_string(),
        field6: 62,
    };
    map.insert(elem1);
}
