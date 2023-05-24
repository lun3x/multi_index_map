use multi_index_map::MultiIndexMap;

#[derive(Hash, PartialEq, Eq, Clone)]
struct TestNonPrimitiveType(u64);

#[derive(MultiIndexMap, Clone)]
struct TestElement {
    #[multi_index(hashed_non_unique)]
    field1: usize,
    field2: usize,
}

#[test]
fn test_non_unique_get_mut() {
    let mut map = MultiIndexTestElementMap::default();
    for i in 0..10 {
        if i % 2 == 0 {
            map.insert(TestElement { field1: 42, field2: i});
        } else {
            map.insert(TestElement { field1: 37, field2: i});
        }
    } 
    let mut_refs = map.get_mut_by_field1(&37);
    for r in mut_refs {
        r.field2 = r.field2 * r.field2;
    }

    let refs = map.get_by_field1(&37);
    for i in 0..5 {
        assert_eq!(refs[i].field2, (2*i+1)*(2*i+1));
    }
}