#[allow(dead_code)]
mod index;
#[allow(dead_code)]
mod order_map;

use order_map::{Order, OrderMap};

fn main() {
    let mut orders = OrderMap::new();
    orders
        .insert(Order::new(1, 100, "John", 25))
        .expect("first order must be unique");
    orders
        .insert(Order::new(2, 90, "John", 30))
        .expect("second order must be unique");
    orders
        .insert(Order::new(3, 110, "Ada", 25))
        .expect("third order must be unique");

    println!("John's orders:");
    for order in orders.by_trader().equal_range("John") {
        println!("  {order:?}");
    }

    println!("Orders by timestamp:");
    for order in orders.by_timestamp().iter() {
        println!("  {order:?}");
    }

    orders
        .by_id_mut()
        .modify(&2, |order| {
            order.timestamp = 120;
            order.price = 25;
        })
        .expect("modification must preserve uniqueness");

    orders.by_trader_mut().update_all("John", |fields| {
        fields.note.push_str("priority");
        *fields.filled = true;
    });

    println!("Orders priced at 25 after mutation:");
    for order in orders.by_price().equal_range(&25) {
        println!("  {order:?}");
    }

    orders.validate().expect("all index invariants must hold");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, HashMap};
    use std::panic::{catch_unwind, AssertUnwindSafe};

    fn populated() -> OrderMap {
        let mut map = OrderMap::new();
        for order in [
            Order::new(1, 100, "John", 25),
            Order::new(2, 90, "John", 30),
            Order::new(3, 110, "Ada", 25),
            Order::new(4, 120, "Grace", 40),
        ] {
            map.insert(order).unwrap();
        }
        map
    }

    #[test]
    fn supports_all_views_and_borrowed_lookup() {
        let map = populated();
        assert_eq!(map.by_id().get(&2).unwrap().trader, "John");
        assert!(map.by_id().contains_key(&4));
        assert_eq!(map.by_trader().equal_range("John").count(), 2);
        assert_eq!(map.by_price().equal_range(&25).count(), 2);
        assert_eq!(
            map.by_timestamp()
                .range(95..=115)
                .map(|order| order.id)
                .collect::<Vec<_>>(),
            vec![1, 3]
        );
        assert_eq!(
            map.by_price()
                .iter()
                .rev()
                .map(|order| order.price)
                .collect::<Vec<_>>(),
            vec![40, 30, 25, 25]
        );
        assert_eq!(map.by_id().iter().len(), 4);
        assert_eq!(map.by_trader().equal_range("John").len(), 2);

        let mut range = map.by_price().range(25..=40);
        assert_eq!(range.next().unwrap().price, 25);
        assert_eq!(range.next_back().unwrap().price, 40);
        assert_eq!(range.count(), 2);
        assert_eq!(map.by_price().range(..25).count(), 0);
        assert_eq!(map.by_price().range(41..).count(), 0);
        assert_eq!(map.by_price().range(31..30).count(), 0);
        map.validate().unwrap();
    }

    #[test]
    fn insertion_checks_unique_indices_without_consuming_stored_values() {
        let mut map = populated();
        let conflict = map.insert(Order::new(1, 999, "Other", 1)).unwrap_err();
        assert_eq!(conflict.index, "id");
        assert_eq!(conflict.value.timestamp, 999);

        let conflict = map.insert(Order::new(99, 100, "Other", 1)).unwrap_err();
        assert_eq!(conflict.index, "timestamp");
        assert_eq!(map.len(), 4);
        map.validate().unwrap();
    }

    #[test]
    fn removal_through_every_index_updates_all_other_indices() {
        let mut map = populated();
        assert_eq!(map.by_id_mut().remove(&1).unwrap().id, 1);
        assert_eq!(map.by_timestamp_mut().remove(&90).unwrap().id, 2);
        assert_eq!(map.by_trader_mut().remove_all("Ada")[0].id, 3);
        assert_eq!(map.by_price_mut().remove_all(&40)[0].id, 4);
        assert!(map.is_empty());
        map.validate().unwrap();
    }

    #[test]
    fn replace_is_atomic_on_conflict() {
        let mut map = populated();
        let replacement = Order::new(1, 90, "Replacement", 5);
        let conflict = map.by_id_mut().replace(&1, replacement).unwrap_err();
        assert_eq!(conflict.index, "timestamp");
        assert_eq!(map.by_id().get(&1).unwrap().timestamp, 100);

        let old = map
            .by_id_mut()
            .replace(&1, Order::new(10, 101, "Replacement", 5))
            .unwrap()
            .unwrap();
        assert_eq!(old.id, 1);
        assert!(map.by_id().get(&1).is_none());
        assert_eq!(map.by_id().get(&10).unwrap().timestamp, 101);
        map.validate().unwrap();
    }

    #[test]
    fn modify_relocates_only_as_needed_and_erases_on_conflict() {
        let mut map = populated();
        map.by_id_mut()
            .modify(&1, |order| {
                order.timestamp = 130;
                order.trader = "Grace".to_string();
                order.price = 50;
            })
            .unwrap();
        assert_eq!(map.by_timestamp().iter().last().unwrap().id, 1);
        assert_eq!(map.by_trader().equal_range("Grace").count(), 2);

        let conflict = map
            .by_id_mut()
            .modify(&1, |order| order.timestamp = 90)
            .unwrap_err();
        assert_eq!(conflict.index, "timestamp");
        assert_eq!(conflict.value.id, 1);
        assert!(!map.by_id().contains_key(&1));
        assert_eq!(map.len(), 3);
        map.validate().unwrap();
    }

    #[test]
    fn panicking_modifier_removes_the_partially_modified_node() {
        let mut map = populated();
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _ = map.by_id_mut().modify(&2, |order| {
                order.price = 999;
                panic!("stop");
            });
        }));
        assert!(result.is_err());
        assert!(!map.by_id().contains_key(&2));
        assert_eq!(map.by_price().equal_range(&999).count(), 0);
        map.validate().unwrap();
    }

    #[test]
    fn batch_mutation_snapshots_original_matches() {
        let mut map = populated();
        let result = map.by_trader_mut().modify_all("John", |order| {
            order.trader = "Moved".to_string();
            order.price += 100;
        });
        assert_eq!(result.modified, 2);
        assert!(result.removed.is_empty());
        assert_eq!(map.by_trader().equal_range("John").count(), 0);
        assert_eq!(map.by_trader().equal_range("Moved").count(), 2);

        assert_eq!(
            map.by_trader_mut().update_all("Moved", |fields| {
                fields.note.push_str("updated");
                *fields.filled = true;
            }),
            2
        );
        assert!(map
            .by_trader()
            .equal_range("Moved")
            .all(|order| order.filled && order.note == "updated"));
        map.validate().unwrap();
    }

    #[test]
    fn complete_typed_view_api_stays_coordinated() {
        let mut map = populated();

        assert_eq!(map.by_id().iter().count(), 4);
        map.by_id_mut().update(&1, |fields| {
            fields.note.push_str("id update");
            *fields.filled = true;
        });
        assert!(map.by_id().get(&1).unwrap().filled);

        assert_eq!(map.by_timestamp().get(&100).unwrap().id, 1);
        assert!(map.by_timestamp().contains_key(&90));
        let old = map
            .by_timestamp_mut()
            .replace(&100, Order::new(10, 101, "Replacement", 5))
            .unwrap()
            .unwrap();
        assert_eq!(old.id, 1);
        map.by_timestamp_mut()
            .modify(&101, |order| order.price = 55)
            .unwrap();
        map.by_timestamp_mut().update(&101, |fields| {
            fields.note.push_str("timestamp update");
        });

        assert_eq!(map.by_trader().iter().count(), 4);
        assert_eq!(
            map.by_price().range(25..=55).map(|order| order.id).count(),
            4
        );
        let result = map
            .by_price_mut()
            .modify_all(&25, |order| order.trader = "At25".to_string());
        assert_eq!(result.modified, 1);
        assert_eq!(
            map.by_price_mut().update_all(&55, |fields| {
                *fields.filled = true;
            }),
            1
        );
        assert!(map.by_id().get(&10).unwrap().filled);
        map.validate().unwrap();
    }

    #[test]
    fn clear_and_slab_slot_reuse_preserve_links() {
        let mut map = populated();
        map.by_id_mut().remove(&2);
        map.insert(Order::new(20, 200, "Reuse", 20)).unwrap();
        assert_eq!(map.by_id().get(&20).unwrap().trader, "Reuse");
        map.validate().unwrap();
        map.clear();
        assert!(map.is_empty());
        map.validate().unwrap();
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct ModelOrder {
        timestamp: u64,
        trader: String,
        price: u64,
    }

    fn assert_model(map: &OrderMap, model: &HashMap<u64, ModelOrder>) {
        map.validate().unwrap();
        assert_eq!(map.len(), model.len());
        for (&id, expected) in model {
            let actual = map.by_id().get(&id).unwrap();
            assert_eq!(actual.timestamp, expected.timestamp);
            assert_eq!(actual.trader, expected.trader);
            assert_eq!(actual.price, expected.price);
        }

        let actual_by_timestamp: Vec<_> = map
            .by_timestamp()
            .iter()
            .map(|order| (order.timestamp, order.id))
            .collect();
        let expected_by_timestamp: Vec<_> = model
            .iter()
            .map(|(&id, order)| (order.timestamp, id))
            .collect::<BTreeMap<_, _>>()
            .into_iter()
            .collect();
        assert_eq!(actual_by_timestamp, expected_by_timestamp);
    }

    #[test]
    fn deterministic_randomized_operations_match_a_simple_model() {
        let mut map = OrderMap::new();
        let mut model = HashMap::new();
        let mut state = 0x9e37_79b9_u64;

        for step in 0..500_u64 {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let id = state % 40;
            match (state >> 8) % 4 {
                0 => {
                    let timestamp = 1_000 + id;
                    let trader = format!("T{}", id % 5);
                    let price = id % 7;
                    if !model.contains_key(&id) {
                        map.insert(Order::new(id, timestamp, trader.clone(), price))
                            .unwrap();
                        model.insert(
                            id,
                            ModelOrder {
                                timestamp,
                                trader,
                                price,
                            },
                        );
                    }
                }
                1 => {
                    let removed = map.by_id_mut().remove(&id);
                    let expected = model.remove(&id);
                    assert_eq!(removed.is_some(), expected.is_some());
                }
                2 => {
                    if let Some(expected) = model.get_mut(&id) {
                        let new_price = (step + id) % 11;
                        map.by_id_mut()
                            .modify(&id, |order| order.price = new_price)
                            .unwrap();
                        expected.price = new_price;
                    }
                }
                _ => {
                    let trader = format!("T{}", id % 5);
                    let expected = model
                        .values_mut()
                        .filter(|order| order.trader == trader)
                        .count();
                    let updated = map.by_trader_mut().update_all(&trader, |fields| {
                        fields.note.push('.');
                    });
                    assert_eq!(updated, expected);
                }
            }
            assert_model(&map, &model);
        }
    }
}
