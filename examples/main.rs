use multi_index_map::MultiIndexMap;
use rustc_hash::FxHashMap;

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
        timestamp: 69,
    };

    let mut map = MultiIndexOrderMap::default();

    // map._id_index.insert(o.id, 0);
    // map._timestamp_index.insert(o.timestamp, 0);
    map.insert(o);

    println!("{map:?}");

    let x = map.get_by_id(&1);
    println!("{x:?}");

    let y = map.get_by_timestamp(&69);
    println!("{y:?}");

    let z = map.remove_by_id(&1);

    println!("{z:?}");

    println!("{map:?}");
}
