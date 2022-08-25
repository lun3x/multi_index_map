use multi_index::MultiIndexClientSubscriptionMap;
use multi_index_map::MultiIndexMap;

#[derive(MultiIndexMap, Clone, Debug)]
struct ClientSubscription {
    #[multi_index(ordered_non_unique)]
    field1: u32,
    #[multi_index(ordered_non_unique)]
    field2: u64,
}

#[test]
fn test_two_ordered_fields() {
    let mut map = MultiIndexClientSubscriptionMap::default();

    map.insert(ClientSubscription {
        field1: 1,
        field2: 999,
    });
    map.insert(ClientSubscription {
        field1: 2,
        field2: 999,
    });

    let a = map.remove_by_field1(&1);

    let b = map.get_by_field2(&999);

    assert_eq!(a.len(), 1);
    assert_eq!(b.len(), 1);
}
