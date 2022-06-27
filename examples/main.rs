use crate::multi_index::MultiIndexOrderMap;
use multi_index_map::MultiIndexMap;

#[derive(MultiIndexMap, Debug)]
struct Order {
    #[multi_index(hashed_unique)]
    id: u32,
    #[multi_index(ordered_unique)]
    timestamp: u64,
    trader_name: String,
}

fn main() {
    let o = Order {
        id: 1,
        timestamp: 11,
        trader_name: "John".to_string(),
    };

    let o2 = Order {
        id: 2,
        timestamp: 22,
        trader_name: "James".to_string(),
    };

    let mut map = MultiIndexOrderMap::default();
    println!("{map:?}");

    map.insert(o);
    println!("{map:?}");

    map.insert(o2);
    println!("{map:?}");

    let w = map.get_by_id(&1).unwrap();
    println!("Got {}'s order by id {}", w.trader_name, w.id);

    let x = map.get_by_timestamp(&11).unwrap();
    println!("Got {}'s order by timestamp {}", x.trader_name, x.timestamp);

    let y = map.remove_by_timestamp(&22).unwrap();
    println!(
        "Removed {}'s order by timestamp {}",
        y.trader_name, y.timestamp
    );
    println!("{map:?}");

    // for order in map.iter_by_timestamp() {}

    let o3 = Order {
        id: 3,
        timestamp: 33,
        trader_name: "Jimbo".to_string(),
    };

    map.insert(o3);
    println!("{map:?}");

    let z = map.remove_by_id(&1).unwrap();
    println!("Removed {}'s order by id {}", z.trader_name, z.id);
    println!("{map:?}");
}
