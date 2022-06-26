use multi_index_map::MultiIndexMap;

#[derive(MultiIndexMap, Debug)]
struct Order {
    // #[multi_index(hashed_unique)]
    id: u32,
    // #[multi_index(ordered_non_unique)]
    timestamp: u64,
}

fn main() {
    let o = Order {
        id: 1,
        timestamp: 11,
    };

    let o2 = Order {
        id: 2,
        timestamp: 22,
    };

    let mut map = MultiIndexOrderMap::default();

    map.insert(o);
    map.insert(o2);

    println!("{map:?}");

    let x = map.get_by_id(&1);
    println!("{x:?}");

    let y = map.get_by_timestamp(&11);
    println!("{y:?}");

    let zz = map.remove_by_timestamp(&22);

    let z = map.remove_by_id(&1);

    println!("{z:?}");

    println!("{map:?}");
}
