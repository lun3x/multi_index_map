use multi_index_map::MultiIndexMap;
use rustc_hash::FxHashMap;

#[derive(MultiIndexMap)]
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

    let map = MultiIndexOrderMap {
        store: Default::default(),
        id_index: Default::default(),
        timestamp_index: Default::default(),
    };
}
