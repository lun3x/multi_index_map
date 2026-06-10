#[allow(dead_code)]
mod order_map;

use order_map::{ById, ByPrice, ByTimestamp, ByTrader, ByTraderTimestamp, Order, OrderMap};

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
    for order in orders.by::<ByTrader>().equal_range("John") {
        println!("  {order:?}");
    }
    println!("John's orders through the compound selector:");
    for order in orders
        .by::<ByTraderTimestamp>()
        .range(("John", &0)..=("John", &u64::MAX))
    {
        println!("  {order:?}");
    }

    println!("Orders by timestamp:");
    for order in orders.by::<ByTimestamp>().iter() {
        println!("  {order:?}");
    }

    orders
        .by_mut::<ById>()
        .modify(&2, |order| {
            order.timestamp = 120;
            order.price = 25;
        })
        .expect("modification must preserve uniqueness");

    orders.by_mut::<ByTrader>().update_all("John", |fields| {
        fields.note.push_str("priority");
        *fields.filled = true;
    });
    orders.by_mut::<ByTimestamp>().update_each(|fields| {
        fields.note.push_str(" timestamp-visited");
    });

    println!("Orders priced at 25 after mutation:");
    for order in orders.by::<ByPrice>().equal_range(&25) {
        println!("  {order:?}");
    }

    orders.validate().expect("all index invariants must hold");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::order_map::{Conflict, ModifyAllResult, OrderUpdate};
    use multi_index_map::{
        IndexView, IndexViewMut, NonUniqueView, NonUniqueViewMut, OrderedView, UniqueView,
        UniqueViewMut,
    };
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
        assert_eq!(map.by::<ById>().get(&2).unwrap().trader, "John");
        assert!(map.by::<ById>().contains_key(&4));
        assert_eq!(map.by::<ByTrader>().equal_range("John").count(), 2);
        assert_eq!(map.by::<ByPrice>().equal_range(&25).count(), 2);
        assert_eq!(
            map.by::<ByTimestamp>()
                .range(95..=115)
                .map(|order| order.id)
                .collect::<Vec<_>>(),
            vec![1, 3]
        );
        assert_eq!(
            map.by::<ByPrice>()
                .iter()
                .rev()
                .map(|order| order.price)
                .collect::<Vec<_>>(),
            vec![40, 30, 25, 25]
        );
        assert_eq!(map.by::<ById>().iter().len(), 4);
        assert_eq!(map.by::<ByTrader>().equal_range("John").len(), 2);

        let mut range = map.by::<ByPrice>().range(25..=40);
        assert_eq!(range.next().unwrap().price, 25);
        assert_eq!(range.next_back().unwrap().price, 40);
        assert_eq!(range.count(), 2);
        assert_eq!(map.by::<ByPrice>().range(..25).count(), 0);
        assert_eq!(map.by::<ByPrice>().range(41..).count(), 0);
        assert_eq!(map.by::<ByPrice>().range(31..30).count(), 0);
        map.validate().unwrap();
    }

    #[test]
    fn compound_selector_supports_full_key_queries_ranges_and_relocation() {
        let mut map = populated();
        assert_eq!(
            map.by::<ByTraderTimestamp>()
                .equal_range(("John", &90))
                .next()
                .unwrap()
                .id,
            2
        );
        assert_eq!(
            map.by::<ByTraderTimestamp>()
                .range(("John", &0)..=("John", &u64::MAX))
                .map(|order| order.id)
                .collect::<Vec<_>>(),
            vec![2, 1]
        );

        map.by_mut::<ById>()
            .modify(&1, |order| {
                order.trader = "Grace".to_owned();
                order.timestamp = 105;
            })
            .unwrap();
        assert_eq!(
            map.by::<ByTraderTimestamp>()
                .equal_range(("Grace", &105))
                .count(),
            1
        );
        assert_eq!(
            map.by_mut::<ByTraderTimestamp>()
                .remove_all(("Grace", &105))
                .len(),
            1
        );
        map.validate().unwrap();
    }

    #[test]
    fn generic_selection_preserves_normal_borrowing() {
        let mut map = populated();
        {
            let ids = map.by::<ById>();
            let timestamps = map.by::<ByTimestamp>();
            assert_eq!(ids.get(&1).unwrap().timestamp, 100);
            assert_eq!(timestamps.get(&100).unwrap().id, 1);
        }
        {
            let mut ids = map.by_mut::<ById>();
            ids.update(&1, |fields| *fields.filled = true);
        }
        assert!(map.by::<ById>().get(&1).unwrap().filled);
    }

    #[test]
    fn capability_traits_support_generic_index_algorithms() {
        fn ids<V>(view: &V) -> Vec<u64>
        where
            V: IndexView<Value = Order>,
        {
            assert_eq!(view.is_empty(), view.len() == 0);
            view.iter().map(|order| order.id).collect()
        }

        fn unique_id<V>(view: &V, key: &V::Key) -> Option<u64>
        where
            V: UniqueView<Value = Order>,
        {
            assert_eq!(view.contains_key(key), view.get(key).is_some());
            view.get(key).map(|order| order.id)
        }

        fn equal_ids<V>(view: &V, key: &V::Key) -> Vec<u64>
        where
            V: NonUniqueView<Value = Order>,
        {
            view.equal_range(key).map(|order| order.id).collect()
        }

        fn range_ids<V>(view: &V, range: impl std::ops::RangeBounds<u64>) -> Vec<u64>
        where
            V: OrderedView<Value = Order, Key = u64>,
        {
            view.range(range).map(|order| order.id).collect()
        }

        fn mutate_unique<V>(view: &mut V, key: &V::Key)
        where
            V: UniqueViewMut<Value = Order, Conflict = Conflict>,
            for<'a> V: IndexViewMut<Update<'a> = OrderUpdate<'a>>,
        {
            view.modify(key, |order| order.price += 1).unwrap();
            view.update(key, |fields| {
                *fields.filled = true;
            });
        }

        fn replace_unique<V>(
            view: &mut V,
            key: &V::Key,
            replacement: Order,
        ) -> Result<Option<Order>, Conflict>
        where
            V: UniqueViewMut<Value = Order, Conflict = Conflict>,
        {
            view.replace(key, replacement)
        }

        fn remove_unique<V>(view: &mut V, key: &V::Key) -> Option<Order>
        where
            V: UniqueViewMut<Value = Order>,
        {
            view.remove(key)
        }

        fn mutate_non_unique<V>(view: &mut V, key: &V::Key) -> ModifyAllResult
        where
            V: NonUniqueViewMut<Value = Order, ModifyAllResult = ModifyAllResult>,
            for<'a> V: IndexViewMut<Update<'a> = OrderUpdate<'a>>,
        {
            let result = view.modify_all(key, |order| order.price += 10);
            view.update_all(key, |fields| {
                fields.note.push_str("trait update");
            });
            result
        }

        fn update_each<V>(view: &mut V) -> usize
        where
            V: IndexViewMut<Value = Order>,
            for<'a> V: IndexViewMut<Update<'a> = OrderUpdate<'a>>,
        {
            view.update_each(|fields| fields.note.push_str(" each"))
        }

        fn remove_non_unique<V>(view: &mut V, key: &V::Key) -> Vec<Order>
        where
            V: NonUniqueViewMut<Value = Order>,
        {
            view.remove_all(key)
        }

        let mut map = populated();
        let john = "John".to_string();
        assert_eq!(ids(&map.by::<ById>()).len(), 4);
        assert_eq!(unique_id(&map.by::<ByTimestamp>(), &100), Some(1));
        let mut john_ids = equal_ids(&map.by::<ByTrader>(), &john);
        john_ids.sort_unstable();
        assert_eq!(john_ids, vec![1, 2]);
        assert_eq!(range_ids(&map.by::<ByPrice>(), 25..=30).len(), 3);

        {
            let mut view = map.by_mut::<ById>();
            assert_eq!(unique_id(&view, &1), Some(1));
            mutate_unique(&mut view, &1);
        }
        assert!(map.by::<ById>().get(&1).unwrap().filled);

        {
            let mut view = map.by_mut::<ByTrader>();
            assert_eq!(equal_ids(&view, &john).len(), 2);
            assert_eq!(mutate_non_unique(&mut view, &john).modified, 2);
            assert_eq!(update_each(&mut view), 4);
        }
        assert!(map
            .by::<ByTrader>()
            .equal_range("John")
            .all(|order| order.note == "trait update each"));

        {
            let view = map.by_mut::<ByPrice>();
            assert_eq!(range_ids(&view, 35..=40).len(), 3);
        }

        {
            let mut view = map.by_mut::<ById>();
            assert_eq!(
                replace_unique(&mut view, &4, Order::new(40, 140, "Replacement", 50))
                    .unwrap()
                    .unwrap()
                    .id,
                4
            );
            assert_eq!(remove_unique(&mut view, &40).unwrap().id, 40);
        }

        {
            let mut view = map.by_mut::<ByPrice>();
            assert_eq!(remove_non_unique(&mut view, &25)[0].id, 3);
        }
        map.validate().unwrap();
    }

    #[test]
    fn insertion_checks_unique_indices_without_consuming_stored_values() {
        let mut map = populated();
        let conflict = map.insert(Order::new(1, 999, "Other", 1)).unwrap_err();
        assert_eq!(conflict.index, "ById");
        assert_eq!(conflict.value.timestamp, 999);

        let conflict = map.insert(Order::new(99, 100, "Other", 1)).unwrap_err();
        assert_eq!(conflict.index, "ByTimestamp");
        assert_eq!(map.len(), 4);
        map.validate().unwrap();
    }

    #[test]
    fn removal_through_every_index_updates_all_other_indices() {
        let mut map = populated();
        assert_eq!(map.by_mut::<ById>().remove(&1).unwrap().id, 1);
        assert_eq!(map.by_mut::<ByTimestamp>().remove(&90).unwrap().id, 2);
        assert_eq!(map.by_mut::<ByTrader>().remove_all("Ada")[0].id, 3);
        assert_eq!(map.by_mut::<ByPrice>().remove_all(&40)[0].id, 4);
        assert!(map.is_empty());
        map.validate().unwrap();
    }

    #[test]
    fn replace_is_atomic_on_conflict() {
        let mut map = populated();
        let replacement = Order::new(1, 90, "Replacement", 5);
        let conflict = map.by_mut::<ById>().replace(&1, replacement).unwrap_err();
        assert_eq!(conflict.index, "ByTimestamp");
        assert_eq!(map.by::<ById>().get(&1).unwrap().timestamp, 100);

        let old = map
            .by_mut::<ById>()
            .replace(&1, Order::new(10, 101, "Replacement", 5))
            .unwrap()
            .unwrap();
        assert_eq!(old.id, 1);
        assert!(map.by::<ById>().get(&1).is_none());
        assert_eq!(map.by::<ById>().get(&10).unwrap().timestamp, 101);
        map.validate().unwrap();
    }

    #[test]
    fn modify_relocates_only_as_needed_and_erases_on_conflict() {
        let mut map = populated();
        map.by_mut::<ById>()
            .modify(&1, |order| {
                order.timestamp = 130;
                order.trader = "Grace".to_string();
                order.price = 50;
            })
            .unwrap();
        assert_eq!(map.by::<ByTimestamp>().iter().last().unwrap().id, 1);
        assert_eq!(map.by::<ByTrader>().equal_range("Grace").count(), 2);

        let conflict = map
            .by_mut::<ById>()
            .modify(&1, |order| order.timestamp = 90)
            .unwrap_err();
        assert_eq!(conflict.index, "ByTimestamp");
        assert_eq!(conflict.value.id, 1);
        assert!(!map.by::<ById>().contains_key(&1));
        assert_eq!(map.len(), 3);
        map.validate().unwrap();
    }

    #[test]
    fn panicking_modifier_removes_the_partially_modified_node() {
        let mut map = populated();
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _ = map.by_mut::<ById>().modify(&2, |order| {
                order.price = 999;
                panic!("stop");
            });
        }));
        assert!(result.is_err());
        assert!(!map.by::<ById>().contains_key(&2));
        assert_eq!(map.by::<ByPrice>().equal_range(&999).count(), 0);
        map.validate().unwrap();
    }

    #[test]
    fn batch_mutation_snapshots_original_matches() {
        let mut map = populated();
        let result = map.by_mut::<ByTrader>().modify_all("John", |order| {
            order.trader = "Moved".to_string();
            order.price += 100;
        });
        assert_eq!(result.modified, 2);
        assert!(result.removed.is_empty());
        assert_eq!(map.by::<ByTrader>().equal_range("John").count(), 0);
        assert_eq!(map.by::<ByTrader>().equal_range("Moved").count(), 2);

        assert_eq!(
            map.by_mut::<ByTrader>().update_all("Moved", |fields| {
                fields.note.push_str("updated");
                *fields.filled = true;
            }),
            2
        );
        assert!(map
            .by::<ByTrader>()
            .equal_range("Moved")
            .all(|order| order.filled && order.note == "updated"));
        map.validate().unwrap();
    }

    #[test]
    fn update_each_visits_the_whole_selected_index_in_index_order() {
        let mut map = populated();
        let mut sequence = 0;
        assert_eq!(
            map.by_mut::<ByTimestamp>().update_each(|fields| {
                *fields.note = sequence.to_string();
                sequence += 1;
            }),
            4
        );
        assert_eq!(
            map.by::<ByTimestamp>()
                .iter()
                .map(|order| order.note.as_str())
                .collect::<Vec<_>>(),
            vec!["0", "1", "2", "3"]
        );

        assert_eq!(
            map.by_mut::<ByTraderTimestamp>()
                .update_each(|fields| *fields.filled = true),
            4
        );
        assert!(map.by::<ById>().iter().all(|order| order.filled));
        map.validate().unwrap();
    }

    #[test]
    fn complete_typed_view_api_stays_coordinated() {
        let mut map = populated();

        assert_eq!(map.by::<ById>().iter().count(), 4);
        map.by_mut::<ById>().update(&1, |fields| {
            fields.note.push_str("id update");
            *fields.filled = true;
        });
        assert!(map.by::<ById>().get(&1).unwrap().filled);

        assert_eq!(map.by::<ByTimestamp>().get(&100).unwrap().id, 1);
        assert!(map.by::<ByTimestamp>().contains_key(&90));
        let old = map
            .by_mut::<ByTimestamp>()
            .replace(&100, Order::new(10, 101, "Replacement", 5))
            .unwrap()
            .unwrap();
        assert_eq!(old.id, 1);
        map.by_mut::<ByTimestamp>()
            .modify(&101, |order| order.price = 55)
            .unwrap();
        map.by_mut::<ByTimestamp>().update(&101, |fields| {
            fields.note.push_str("timestamp update");
        });

        assert_eq!(map.by::<ByTrader>().iter().count(), 4);
        assert_eq!(
            map.by::<ByPrice>()
                .range(25..=55)
                .map(|order| order.id)
                .count(),
            4
        );
        let result = map
            .by_mut::<ByPrice>()
            .modify_all(&25, |order| order.trader = "At25".to_string());
        assert_eq!(result.modified, 1);
        assert_eq!(
            map.by_mut::<ByPrice>().update_all(&55, |fields| {
                *fields.filled = true;
            }),
            1
        );
        assert!(map.by::<ById>().get(&10).unwrap().filled);
        map.validate().unwrap();
    }

    #[test]
    fn clear_and_slab_slot_reuse_preserve_links() {
        let mut map = populated();
        map.by_mut::<ById>().remove(&2);
        map.insert(Order::new(20, 200, "Reuse", 20)).unwrap();
        assert_eq!(map.by::<ById>().get(&20).unwrap().trader, "Reuse");
        map.validate().unwrap();
        map.clear();
        assert!(map.is_empty());
        map.validate().unwrap();
    }

    #[allow(deprecated)]
    mod compatibility {
        use super::*;

        fn sorted_ids(orders: Vec<&Order>) -> Vec<u64> {
            let mut ids = orders.into_iter().map(|order| order.id).collect::<Vec<_>>();
            ids.sort_unstable();
            ids
        }

        #[test]
        fn field_named_getters_and_iterators_wrap_all_index_kinds() {
            let map = populated();
            let john = "John".to_string();

            assert_eq!(map.get_by_id(&2).unwrap().trader, "John");
            assert_eq!(map.get_by_timestamp(&100).unwrap().id, 1);
            assert_eq!(sorted_ids(map.get_by_trader("John")), vec![1, 2]);
            assert_eq!(sorted_ids(map.get_by_trader(&john)), vec![1, 2]);
            assert_eq!(sorted_ids(map.get_by_price(&25)), vec![1, 3]);

            assert_eq!(map.iter_by_id().count(), 4);
            assert_eq!(
                map.iter_by_timestamp()
                    .map(|order| order.timestamp)
                    .collect::<Vec<_>>(),
                vec![90, 100, 110, 120]
            );
            assert_eq!(map.iter_by_trader().count(), 4);
            assert_eq!(
                map.iter_by_price()
                    .rev()
                    .map(|order| order.price)
                    .collect::<Vec<_>>(),
                vec![40, 30, 25, 25]
            );
        }

        #[test]
        fn field_named_get_mut_accesses_only_unindexed_fields_with_holes() {
            let mut map = populated();
            map.remove_by_id(&2);
            map.insert(Order::new(20, 200, "John", 25)).unwrap();

            {
                let (note, filled) = map.get_mut_by_id(&20).unwrap();
                note.push_str("id");
                *filled = true;
            }
            {
                let (note, _) = map.get_mut_by_timestamp(&100).unwrap();
                note.push_str(" timestamp");
            }
            {
                let john = "John".to_string();
                for (note, filled) in map.get_mut_by_trader(&john) {
                    note.push_str(" trader");
                    *filled = true;
                }
            }
            for (note, _) in map.get_mut_by_price(&25) {
                note.push_str(" price");
            }

            assert_eq!(map.by::<ById>().get(&20).unwrap().note, "id trader price");
            assert!(map.by::<ById>().get(&20).unwrap().filled);
            assert_eq!(
                map.by::<ById>().get(&1).unwrap().note,
                " timestamp trader price"
            );
            assert_eq!(map.by::<ById>().get(&3).unwrap().note, " price");
            map.validate().unwrap();
        }

        #[test]
        fn iter_mut_visits_unindexed_fields_in_slab_order() {
            fn assert_fused<I: std::iter::FusedIterator>(_: &I) {}

            let mut map = populated();
            map.remove_by_id(&2);
            map.insert(Order::new(20, 200, "Reuse", 20)).unwrap();

            let mut iter = map.iter_mut();
            assert_fused(&iter);
            assert_eq!(iter.len(), 4);
            *iter.next().unwrap().0 = "front".to_owned();
            *iter.next_back().unwrap().0 = "back".to_owned();
            *iter.next().unwrap().0 = "reused".to_owned();
            *iter.next().unwrap().0 = "middle".to_owned();
            assert!(iter.next().is_none());
            assert!(iter.next().is_none());
            drop(iter);

            assert_eq!(map.by::<ById>().get(&1).unwrap().note, "front");
            assert_eq!(map.by::<ById>().get(&20).unwrap().note, "reused");
            assert_eq!(map.by::<ById>().get(&3).unwrap().note, "middle");
            assert_eq!(map.by::<ById>().get(&4).unwrap().note, "back");
            map.validate().unwrap();
        }

        #[test]
        fn field_named_updates_preserve_legacy_closure_and_return_shapes() {
            let mut map = populated();
            let john = "John".to_string();

            let order = map
                .update_by_id(&1, |note, filled| {
                    note.push_str("id");
                    *filled = true;
                })
                .unwrap();
            assert_eq!(order.id, 1);

            let order = map
                .update_by_timestamp(&90, |note, _| note.push_str(" timestamp"))
                .unwrap();
            assert_eq!(order.id, 2);

            assert_eq!(
                sorted_ids(map.update_by_trader("John", |note, _| note.push_str(" str"))),
                vec![1, 2]
            );
            assert_eq!(
                sorted_ids(map.update_by_trader(&john, |note, _| note.push_str(" string"))),
                vec![1, 2]
            );
            assert_eq!(
                sorted_ids(map.update_by_price(&25, |note, _| note.push_str(" price"))),
                vec![1, 3]
            );

            assert_eq!(
                map.by::<ById>().get(&1).unwrap().note,
                "id str string price"
            );
            assert_eq!(
                map.by::<ById>().get(&2).unwrap().note,
                " timestamp str string"
            );
            map.validate().unwrap();
        }

        #[test]
        fn field_named_modifiers_relocate_and_process_original_batches_once() {
            let mut map = populated();

            let modified = map
                .modify_by_id(&1, |order| {
                    order.timestamp = 130;
                    order.trader = "Moved".to_string();
                    order.price = 50;
                })
                .unwrap();
            assert_eq!(modified.id, 1);

            let modified = map
                .modify_by_timestamp(&90, |order| order.price = 25)
                .unwrap();
            assert_eq!(modified.id, 2);

            let john = "John".to_string();
            let mut trader_calls = 0;
            let modified = map.modify_by_trader(&john, |order| {
                trader_calls += 1;
                order.trader = "Moved".to_string();
            });
            assert_eq!(trader_calls, 1);
            assert_eq!(sorted_ids(modified), vec![2]);

            let mut price_calls = 0;
            let modified = map.modify_by_price(&25, |order| {
                price_calls += 1;
                order.price = 60;
            });
            assert_eq!(price_calls, 2);
            assert_eq!(sorted_ids(modified), vec![2, 3]);
            assert_eq!(map.by::<ByPrice>().equal_range(&25).count(), 0);
            assert_eq!(map.by::<ByPrice>().equal_range(&60).count(), 2);
            map.validate().unwrap();
        }

        #[test]
        fn compatibility_modifiers_panic_after_new_conflict_cleanup() {
            let mut map = populated();
            let unique_result = catch_unwind(AssertUnwindSafe(|| {
                map.modify_by_id(&2, |order| order.timestamp = 100);
            }));
            assert!(unique_result.is_err());
            assert!(!map.by::<ById>().contains_key(&2));
            map.validate().unwrap();

            let mut map = populated();
            let john = "John".to_string();
            let mut calls = 0;
            let batch_result = catch_unwind(AssertUnwindSafe(|| {
                map.modify_by_trader(&john, |order| {
                    calls += 1;
                    order.timestamp = 90;
                });
            }));
            assert!(batch_result.is_err());
            assert_eq!(calls, 2);
            assert!(!map.by::<ById>().contains_key(&1));
            assert!(map.by::<ById>().contains_key(&2));
            map.validate().unwrap();
        }

        #[test]
        fn field_named_removers_wrap_all_index_kinds() {
            let mut map = populated();
            let ada = "Ada".to_string();

            assert_eq!(map.remove_by_id(&1).unwrap().id, 1);
            assert_eq!(map.remove_by_timestamp(&90).unwrap().id, 2);
            assert_eq!(map.remove_by_trader(&ada)[0].id, 3);
            assert_eq!(map.remove_by_price(&40)[0].id, 4);
            assert!(map.is_empty());
            map.validate().unwrap();
        }
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
            let actual = map.by::<ById>().get(&id).unwrap();
            assert_eq!(actual.timestamp, expected.timestamp);
            assert_eq!(actual.trader, expected.trader);
            assert_eq!(actual.price, expected.price);
        }

        let actual_by_timestamp: Vec<_> = map
            .by::<ByTimestamp>()
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
                    let removed = map.by_mut::<ById>().remove(&id);
                    let expected = model.remove(&id);
                    assert_eq!(removed.is_some(), expected.is_some());
                }
                2 => {
                    if let Some(expected) = model.get_mut(&id) {
                        let new_price = (step + id) % 11;
                        map.by_mut::<ById>()
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
                    let updated = map.by_mut::<ByTrader>().update_all(&trader, |fields| {
                        fields.note.push('.');
                    });
                    assert_eq!(updated, expected);
                }
            }
            assert_model(&map, &model);
        }
    }
}
