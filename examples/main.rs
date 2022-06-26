use crate::multi_index::MultiIndexOrderMap;
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
    println!("{map:?}");

    map.insert(o);
    println!("{map:?}");

    map.insert(o2);
    println!("{map:?}");

    let w = map.get_by_id(&1);
    println!("{w:?}");

    let x = map.get_by_timestamp(&11);
    println!("{x:?}");

    let y = map.remove_by_timestamp(&22);
    println!("{y:?}");
    println!("{map:?}");

    let o3 = Order {
        id: 3,
        timestamp: 33,
    };

    map.insert(o3);
    println!("{map:?}");

    let z = map.remove_by_id(&1);
    println!("{z:?}");
    println!("{map:?}");
}
