use crate::multi_index_order::MultiIndexOrderMap;
use multi_index_map::MultiIndexMap;

#[derive(MultiIndexMap, Debug, Clone)]
struct Order {
    #[multi_index(hashed_unique)]
    order_id: u32,
    #[multi_index(ordered_unique)]
    timestamp: u64,
    #[multi_index(hashed_non_unique)]
    trader_name: String,
    note: String,
}

fn main() {
    let o1 = Order {
        order_id: 1,
        timestamp: 111,
        trader_name: "John".to_string(),
        note: "".to_string(),
    };

    let o2 = Order {
        order_id: 2,
        timestamp: 22,
        trader_name: "Tom".to_string(),
        note: "".to_string(),
    };

    let mut map = MultiIndexOrderMap::default();

    map.insert(o1);
    map.insert(o2);

    // Set non-mutable, non mutating iter methods still work.
    let map = map;

    for o in map.iter_by_timestamp() {
        println!("iter_by_timestamp: {o:?}")
    }

    for o in map.iter_by_order_id() {
        println!("iter_by_order_id: {o:?}")
    }

    for (_, o) in map.iter() {
        println!("iter: {o:?}")
    }

    let o1_ref = map.get_by_order_id(&1).unwrap();
    println!(
        "Got {}'s order by id {}",
        o1_ref.trader_name, o1_ref.order_id
    );

    // Set mutable so we can mutate the map.
    let mut map = map;

    for (_, o) in unsafe { map.iter_mut() } {
        println!("iter_mut: {o:?}")
    }

    let o1_ref = map
        .modify_by_order_id(&1, |o| {
            o.order_id = 7;
            o.timestamp = 77;
            o.trader_name = "Tom".to_string();
        })
        .unwrap();
    println!(
        "Modified {}'s order by id, to {:?}",
        o1_ref.trader_name, o1_ref
    );

    let o1_mut_ref = unsafe { map.get_mut_by_order_id(&7).unwrap() };
    o1_mut_ref.note = "TestNote".to_string();
    println!(
        "Changed note of order {o1_mut_ref:?}, to {:?}",
        o1_mut_ref.note,
    );

    let toms_orders = map.remove_by_trader_name(&"Tom".to_string());
    assert_eq!(toms_orders.len(), 2);
    println!("Removed Tom's order by name: {toms_orders:?}",);

    let o3 = Order {
        order_id: 3,
        timestamp: 33,
        trader_name: "Jimbo".to_string(),
        note: "".to_string(),
    };

    map.insert(o3);
    let o3 = map.remove_by_timestamp(&33).unwrap();
    println!(
        "Removed {}'s order by timestamp {}",
        o3.trader_name, o3.timestamp
    );
}
