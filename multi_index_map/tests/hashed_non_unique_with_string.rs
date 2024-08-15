use multi_index_map::MultiIndexMap;

#[derive(MultiIndexMap, Debug)]
#[multi_index_derive(Debug)]
pub(crate) struct Order {
    #[multi_index(hashed_unique)]
    pub(crate) order_id: u32,
    #[multi_index(ordered_non_unique)]
    pub(crate) timestamp: u64,
    #[multi_index(hashed_non_unique)]
    pub(crate) trader_name: String,
}

#[test]
fn iter_after_modify() {
    let o1 = Order {
        order_id: 1,
        timestamp: 111,
        trader_name: "John".to_string(),
    };

    let o2 = Order {
        order_id: 2,
        timestamp: 22,
        trader_name: "Mike".to_string(),
    };

    let o3 = Order {
        order_id: 3,
        timestamp: 33,
        trader_name: "Tom".to_string(),
    };

    let o4 = Order {
        order_id: 4,
        timestamp: 44,
        trader_name: "Jerry".to_string(),
    };

    let mut map = MultiIndexOrderMap::default();

    map.insert(o1);
    map.insert(o2);
    map.insert(o3);
    map.insert(o4);

    {
        let mut it = map.iter_by_timestamp();
        assert_eq!(it.next().unwrap().order_id, 2);
        assert_eq!(it.next().unwrap().order_id, 3);
        assert_eq!(it.next().unwrap().order_id, 4);
        assert_eq!(it.next().unwrap().order_id, 1);
    }

    let mut s = "test".to_string();

    map.modify_by_order_id(&1, |o| {
        s = "p".to_string();
        o.timestamp = 4;
    });

    assert_eq!(s, "p");

    {
        let mut it = map.iter_by_timestamp();
        assert_eq!(it.next().unwrap().order_id, 1);
        assert_eq!(it.next().unwrap().order_id, 2);
        assert_eq!(it.next().unwrap().order_id, 3);
        assert_eq!(it.next().unwrap().order_id, 4);
    }
}

#[test]
fn get_by_borrowed_string() {
    let o1 = Order {
        order_id: 1,
        timestamp: 111,
        trader_name: "John".to_string(),
    };

    let mut map = MultiIndexOrderMap::default();

    map.insert(o1);

    let res = map.get_by_trader_name("John");
    assert_eq!(res.len(), 1);
    assert_eq!(res.first().unwrap().order_id, 1);
}
