use crate::multi_index::MultiIndexOrderMap;
use multi_index_map::MultiIndexMap;

#[derive(MultiIndexMap, Debug, Clone)]
struct Order {
    #[multi_index(hashed_unique)]
    order_id: u32,
    #[multi_index(ordered_unique)]
    timestamp: u64,
    trader_name: String,
}

fn main() {
    let o = Order {
        order_id: 1,
        timestamp: 111,
        trader_name: "John".to_string(),
    };

    let o2 = Order {
        order_id: 2,
        timestamp: 22,
        trader_name: "James".to_string(),
    };

    let mut map = MultiIndexOrderMap::default();

    map.insert(o);
    map.insert(o2);

    for i in map.iter_by_timestamp() {
        println!("iter_by_timestamp: {i:?}")
    }

    for i in map.iter_by_order_id() {
        println!("iter_by_order_id: {i:?}")
    }

    for i in map.iter() {
        println!("iter: {i:?}")
    }

    for i in unsafe { map.iter_mut() } {
        println!("iter_mut: {i:?}")
    }

    let w = map.get_by_order_id(&1).unwrap();
    println!("Got {}'s order by id {}", w.trader_name, w.order_id);

    let x = map
        .modify_by_order_id(&1, |o| {
            o.order_id = 7;
            o.timestamp = 77
        })
        .unwrap();
    println!("Modified {}'s order by id, to {:?}", x.trader_name, x);

    let y = map.remove_by_timestamp(&22).unwrap();
    println!(
        "Removed {}'s order by timestamp {}",
        y.trader_name, y.timestamp
    );

    let o3 = Order {
        order_id: 3,
        timestamp: 33,
        trader_name: "Jimbo".to_string(),
    };

    map.insert(o3);
    let z = map.remove_by_timestamp(&77).unwrap();
    println!(
        "Removed {}'s order by timestamp {}",
        z.trader_name, z.timestamp
    );
}
