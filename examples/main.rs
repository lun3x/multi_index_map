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
        id: 0,
        timestamp: 0,
    };

    let map = MultiIndexOrderMap::default();

    println!("{map:?}");

    let x = map.get_by_id(&0);
    println!("{x:?}")
}
