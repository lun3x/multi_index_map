use multi_index_map::MultiIndexMap;

#[derive(MultiIndexMap, Debug)]
struct TestElement {
    #[multi_index(hashed_unique)]
    field1: i32,
    #[multi_index(hashed_non_unique)]
    field2: i32,
    #[multi_index(ordered_unique)]
    field3: i32,
    #[multi_index(ordered_non_unique)]
    field4: i32,
}

#[test]
fn test_initialize_with_capacity() {
    let map = MultiIndexTestElementMap::with_capacity(10);
    assert_eq!(map.capacity(), 10);
    assert_eq!(map.len(), 0);
}

#[test]
fn test_reserve() {
    let mut map = MultiIndexTestElementMap::default();
    assert_eq!(map.capacity(), 0);
    map.reserve(10);
    assert_eq!(map.capacity(), 10);
}

#[test]
fn test_shrink() {
    let mut map = MultiIndexTestElementMap::default();
    for i in 0..10 {
        map.insert(TestElement {
            field1: i,
            field2: i,
            field3: i,
            field4: i,
        });
    }
    map.shrink_to_fit();
    assert_eq!(map.capacity(), 10);
    map.reserve(10);
    assert_eq!(map.capacity(), 20);
    map.shrink_to_fit();
    assert_eq!(map.capacity(), 10);

    map.remove_by_field1(&5);
    map.remove_by_field1(&6);
    map.shrink_to_fit();
    assert_eq!(map.capacity(), 10);

    for i in 11..14 {
        map.insert(TestElement {
            field1: i,
            field2: i,
            field3: i,
            field4: i,
        });
    }

    map.shrink_to_fit();
    assert_eq!(map.capacity(), 11);
}
