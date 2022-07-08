use crate::multi_index::MultiIndexOrderMap;
use multi_index_map::MultiIndexMap;

#[derive(MultiIndexMap, Debug, Clone)]
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
        timestamp: 111,
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

    for i in map.iter_by_timestamp() {
        println!("iter_by_timestamp: {i:?}")
    }

    for i in map.iter_by_id() {
        println!("iter_by_id: {i:?}")
    }

    let w = map.get_by_id(&1).unwrap();
    println!("Got {}'s order by id {}", w.trader_name, w.id);

    let x = map
        .modify_by_id(&1, |o| {
            o.id = 7;
            o.timestamp = 77
        })
        .unwrap();
    println!("Modified {}'s order by id, to {:?}", x.trader_name, x);

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

    let z = map.remove_by_timestamp(&77).unwrap();
    println!(
        "Removed {}'s order by timestamp {}",
        z.trader_name, z.timestamp
    );
    println!("{map:?}");
}