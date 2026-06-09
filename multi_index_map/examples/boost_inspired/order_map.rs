use multi_index_map::__private::{
    HashEqualIds, HashIds, HashLink, HashSpec, HashedIndex, NodeId, NodeValue, OrderedIds,
    OrderedIndex, OrderedLink, OrderedRangeIds, OrderedSpec, Slab,
};
use multi_index_map::{Conflict as GenericConflict, ModifyAllResult as GenericModifyAllResult};
use multi_index_map::{
    IndexView, NonUniqueView, NonUniqueViewMut, OrderedView, UniqueView, UniqueViewMut,
};
use std::borrow::Borrow;
use std::hash::Hash;
use std::ops::RangeBounds;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Order {
    pub(crate) id: u64,
    pub(crate) timestamp: u64,
    pub(crate) trader: String,
    pub(crate) price: u64,
    pub(crate) note: String,
    pub(crate) filled: bool,
}

impl Order {
    pub(crate) fn new(id: u64, timestamp: u64, trader: impl Into<String>, price: u64) -> Self {
        Self {
            id,
            timestamp,
            trader: trader.into(),
            price,
            note: String::new(),
            filled: false,
        }
    }
}

pub(crate) type Conflict = GenericConflict<Order>;

pub(crate) struct OrderUpdate<'a> {
    pub(crate) note: &'a mut String,
    pub(crate) filled: &'a mut bool,
}

pub(crate) type ModifyAllResult = GenericModifyAllResult<Order>;

#[derive(Debug)]
struct OrderNode {
    order: Order,
    id_link: HashLink,
    timestamp_link: OrderedLink,
    trader_link: HashLink,
    price_link: OrderedLink,
}

impl OrderNode {
    fn new(order: Order) -> Self {
        Self {
            order,
            id_link: HashLink::default(),
            timestamp_link: OrderedLink::default(),
            trader_link: HashLink::default(),
            price_link: OrderedLink::default(),
        }
    }
}

impl NodeValue for OrderNode {
    type Value = Order;

    fn value(&self) -> &Self::Value {
        &self.order
    }
}

struct ById;
struct ByTimestamp;
struct ByTrader;
struct ByPrice;

impl HashSpec<OrderNode> for ById {
    type Key = u64;

    const NAME: &'static str = "id";

    fn key(value: &Order) -> &Self::Key {
        &value.id
    }

    fn link(node: &OrderNode) -> &HashLink {
        &node.id_link
    }

    fn link_mut(node: &mut OrderNode) -> &mut HashLink {
        &mut node.id_link
    }
}

impl OrderedSpec<OrderNode> for ByTimestamp {
    type Key = u64;

    const NAME: &'static str = "timestamp";

    fn key(value: &Order) -> &Self::Key {
        &value.timestamp
    }

    fn link(node: &OrderNode) -> &OrderedLink {
        &node.timestamp_link
    }

    fn link_mut(node: &mut OrderNode) -> &mut OrderedLink {
        &mut node.timestamp_link
    }
}

impl HashSpec<OrderNode> for ByTrader {
    type Key = String;

    const NAME: &'static str = "trader";

    fn key(value: &Order) -> &Self::Key {
        &value.trader
    }

    fn link(node: &OrderNode) -> &HashLink {
        &node.trader_link
    }

    fn link_mut(node: &mut OrderNode) -> &mut HashLink {
        &mut node.trader_link
    }
}

impl OrderedSpec<OrderNode> for ByPrice {
    type Key = u64;

    const NAME: &'static str = "price";

    fn key(value: &Order) -> &Self::Key {
        &value.price
    }

    fn link(node: &OrderNode) -> &OrderedLink {
        &node.price_link
    }

    fn link_mut(node: &mut OrderNode) -> &mut OrderedLink {
        &mut node.price_link
    }
}

type IdIndex = HashedIndex<ById, true>;
type TimestampIndex = OrderedIndex<ByTimestamp, true>;
type TraderIndex = HashedIndex<ByTrader, false>;
type PriceIndex = OrderedIndex<ByPrice, false>;

#[derive(Default)]
pub(crate) struct OrderMap {
    nodes: Slab<OrderNode>,
    id: IdIndex,
    timestamp: TimestampIndex,
    trader: TraderIndex,
    price: PriceIndex,
}

impl OrderMap {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn len(&self) -> usize {
        self.nodes.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub(crate) fn insert(&mut self, order: Order) -> Result<&Order, Conflict> {
        if self.id.find(&order.id, &self.nodes).is_some() {
            return Err(Conflict {
                index: ById::NAME,
                value: order,
            });
        }
        if self.timestamp.find(&order.timestamp, &self.nodes).is_some() {
            return Err(Conflict {
                index: ByTimestamp::NAME,
                value: order,
            });
        }

        self.id.reserve_for_insert(&mut self.nodes);
        self.trader.reserve_for_insert(&mut self.nodes);
        let id = NodeId(self.nodes.insert(OrderNode::new(order)));
        self.link_all(id);
        self.validate_debug();
        Ok(&self.nodes[id.0].order)
    }

    pub(crate) fn clear(&mut self) {
        let ids: Vec<_> = self.nodes.iter().map(|(id, _)| NodeId(id)).collect();
        for id in ids {
            self.remove_id(id);
        }
        self.validate_debug();
    }

    pub(crate) fn by_id(&self) -> IdView<'_> {
        IdView { map: self }
    }

    pub(crate) fn by_id_mut(&mut self) -> IdViewMut<'_> {
        IdViewMut { map: self }
    }

    pub(crate) fn by_timestamp(&self) -> TimestampView<'_> {
        TimestampView { map: self }
    }

    pub(crate) fn by_timestamp_mut(&mut self) -> TimestampViewMut<'_> {
        TimestampViewMut { map: self }
    }

    pub(crate) fn by_trader(&self) -> TraderView<'_> {
        TraderView { map: self }
    }

    pub(crate) fn by_trader_mut(&mut self) -> TraderViewMut<'_> {
        TraderViewMut { map: self }
    }

    pub(crate) fn by_price(&self) -> PriceView<'_> {
        PriceView { map: self }
    }

    pub(crate) fn by_price_mut(&mut self) -> PriceViewMut<'_> {
        PriceViewMut { map: self }
    }

    fn update_fields_for_ids(&mut self, mut ids: Vec<NodeId>) -> Vec<(&mut String, &mut bool)> {
        ids.sort_unstable_by_key(|id| id.0);
        assert!(
            !ids.windows(2).any(|pair| pair[0] == pair[1]),
            "compatibility accessor received a duplicate arena node"
        );

        let mut fields = Vec::with_capacity(ids.len());
        let mut targets = ids.into_iter();
        let mut target = targets.next();
        for (slot, node) in self.nodes.iter_mut() {
            if target.map(|id| id.0) == Some(slot) {
                fields.push((&mut node.order.note, &mut node.order.filled));
                target = targets.next();
            }
        }
        assert!(
            target.is_none(),
            "compatibility accessor targeted a missing arena node"
        );
        fields
    }

    fn order_refs_for_ids(&self, ids: &[NodeId]) -> Vec<&Order> {
        ids.iter()
            .map(|id| {
                &self
                    .nodes
                    .get(id.0)
                    .expect("compatibility accessor targeted a missing arena node")
                    .order
            })
            .collect()
    }

    fn panic_on_modify_conflicts(result: ModifyAllResult) {
        if let Some(conflict) = result.removed.first() {
            panic!(
                "compatibility modifier removed {} element(s) after uniqueness conflict on index '{}'",
                result.removed.len(),
                conflict.index
            );
        }
    }

    #[deprecated(note = "use by_id().get(key)")]
    pub(crate) fn get_by_id<Q>(&self, key: &Q) -> Option<&Order>
    where
        u64: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.id
            .find(key, &self.nodes)
            .map(|id| &self.nodes[id.0].order)
    }

    #[deprecated(note = "use by_id_mut().update(key, ...)")]
    pub(crate) fn get_mut_by_id(&mut self, key: &u64) -> Option<(&mut String, &mut bool)> {
        let id = self.id.find(key, &self.nodes)?;
        self.update_fields_for_ids(vec![id]).into_iter().next()
    }

    #[deprecated(note = "use by_id_mut().modify(key, ...)")]
    pub(crate) fn modify_by_id(&mut self, key: &u64, f: impl FnOnce(&mut Order)) -> Option<&Order> {
        let id = self.id.find(key, &self.nodes)?;
        match self.modify_id(id, f) {
            Ok(order) => Some(order),
            Err(conflict) => panic!(
                "compatibility modifier removed an element after uniqueness conflict on index '{}'",
                conflict.index
            ),
        }
    }

    #[deprecated(note = "use by_id_mut().update(key, ...)")]
    pub(crate) fn update_by_id<Q>(
        &mut self,
        key: &Q,
        f: impl FnOnce(&mut String, &mut bool),
    ) -> Option<&Order>
    where
        u64: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        let id = self.id.find(key, &self.nodes)?;
        Some(self.update_id(id, |fields| f(fields.note, fields.filled)))
    }

    #[deprecated(note = "use by_id_mut().remove(key)")]
    pub(crate) fn remove_by_id(&mut self, key: &u64) -> Option<Order> {
        self.by_id_mut().remove(key)
    }

    #[deprecated(note = "use by_id().iter()")]
    pub(crate) fn iter_by_id(&self) -> IdIter<'_> {
        IdIter::new(OrderRefs::new(&self.nodes, self.id.iter_ids(&self.nodes)))
    }

    #[deprecated(note = "use by_timestamp().get(key)")]
    pub(crate) fn get_by_timestamp<Q>(&self, key: &Q) -> Option<&Order>
    where
        u64: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.timestamp
            .find(key, &self.nodes)
            .map(|id| &self.nodes[id.0].order)
    }

    #[deprecated(note = "use by_timestamp_mut().update(key, ...)")]
    pub(crate) fn get_mut_by_timestamp(&mut self, key: &u64) -> Option<(&mut String, &mut bool)> {
        let id = self.timestamp.find(key, &self.nodes)?;
        self.update_fields_for_ids(vec![id]).into_iter().next()
    }

    #[deprecated(note = "use by_timestamp_mut().modify(key, ...)")]
    pub(crate) fn modify_by_timestamp(
        &mut self,
        key: &u64,
        f: impl FnOnce(&mut Order),
    ) -> Option<&Order> {
        let id = self.timestamp.find(key, &self.nodes)?;
        match self.modify_id(id, f) {
            Ok(order) => Some(order),
            Err(conflict) => panic!(
                "compatibility modifier removed an element after uniqueness conflict on index '{}'",
                conflict.index
            ),
        }
    }

    #[deprecated(note = "use by_timestamp_mut().update(key, ...)")]
    pub(crate) fn update_by_timestamp<Q>(
        &mut self,
        key: &Q,
        f: impl FnOnce(&mut String, &mut bool),
    ) -> Option<&Order>
    where
        u64: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        let id = self.timestamp.find(key, &self.nodes)?;
        Some(self.update_id(id, |fields| f(fields.note, fields.filled)))
    }

    #[deprecated(note = "use by_timestamp_mut().remove(key)")]
    pub(crate) fn remove_by_timestamp(&mut self, key: &u64) -> Option<Order> {
        self.by_timestamp_mut().remove(key)
    }

    #[deprecated(note = "use by_timestamp().iter()")]
    pub(crate) fn iter_by_timestamp(&self) -> TimestampIter<'_> {
        TimestampIter::new(OrderRefs::new(
            &self.nodes,
            self.timestamp.iter_ids(&self.nodes),
        ))
    }

    #[deprecated(note = "use by_trader().equal_range(key)")]
    pub(crate) fn get_by_trader<Q>(&self, key: &Q) -> Vec<&Order>
    where
        String: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.by_trader().equal_range(key).collect()
    }

    #[deprecated(note = "use by_trader_mut().update_all(key, ...)")]
    pub(crate) fn get_mut_by_trader(&mut self, key: &String) -> Vec<(&mut String, &mut bool)> {
        let ids = self.trader.equal_ids(key, &self.nodes);
        self.update_fields_for_ids(ids)
    }

    #[deprecated(note = "use by_trader_mut().modify_all(key, ...)")]
    pub(crate) fn modify_by_trader(
        &mut self,
        key: &String,
        f: impl FnMut(&mut Order),
    ) -> Vec<&Order> {
        let ids = self.trader.equal_ids(key, &self.nodes);
        let result = self.by_trader_mut().modify_all(key, f);
        Self::panic_on_modify_conflicts(result);
        self.order_refs_for_ids(&ids)
    }

    #[deprecated(note = "use by_trader_mut().update_all(key, ...)")]
    pub(crate) fn update_by_trader<Q>(
        &mut self,
        key: &Q,
        mut f: impl FnMut(&mut String, &mut bool),
    ) -> Vec<&Order>
    where
        String: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        let ids = self.trader.equal_ids(key, &self.nodes);
        self.by_trader_mut()
            .update_all(key, |fields| f(fields.note, fields.filled));
        self.order_refs_for_ids(&ids)
    }

    #[deprecated(note = "use by_trader_mut().remove_all(key)")]
    pub(crate) fn remove_by_trader(&mut self, key: &String) -> Vec<Order> {
        self.by_trader_mut().remove_all(key)
    }

    #[deprecated(note = "use by_trader().iter()")]
    pub(crate) fn iter_by_trader(&self) -> TraderIter<'_> {
        TraderIter::new(OrderRefs::new(
            &self.nodes,
            self.trader.iter_ids(&self.nodes),
        ))
    }

    #[deprecated(note = "use by_price().equal_range(key)")]
    pub(crate) fn get_by_price<Q>(&self, key: &Q) -> Vec<&Order>
    where
        u64: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.price
            .equal_ids(key, &self.nodes)
            .into_iter()
            .map(|id| &self.nodes[id.0].order)
            .collect()
    }

    #[deprecated(note = "use by_price_mut().update_all(key, ...)")]
    pub(crate) fn get_mut_by_price(&mut self, key: &u64) -> Vec<(&mut String, &mut bool)> {
        let ids = self.price.equal_ids(key, &self.nodes);
        self.update_fields_for_ids(ids)
    }

    #[deprecated(note = "use by_price_mut().modify_all(key, ...)")]
    pub(crate) fn modify_by_price(&mut self, key: &u64, f: impl FnMut(&mut Order)) -> Vec<&Order> {
        let ids = self.price.equal_ids(key, &self.nodes);
        let result = self.by_price_mut().modify_all(key, f);
        Self::panic_on_modify_conflicts(result);
        self.order_refs_for_ids(&ids)
    }

    #[deprecated(note = "use by_price_mut().update_all(key, ...)")]
    pub(crate) fn update_by_price<Q>(
        &mut self,
        key: &Q,
        mut f: impl FnMut(&mut String, &mut bool),
    ) -> Vec<&Order>
    where
        u64: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        let ids = self.price.equal_ids(key, &self.nodes);
        for id in &ids {
            self.update_id(*id, |fields| f(fields.note, fields.filled));
        }
        self.order_refs_for_ids(&ids)
    }

    #[deprecated(note = "use by_price_mut().remove_all(key)")]
    pub(crate) fn remove_by_price(&mut self, key: &u64) -> Vec<Order> {
        self.by_price_mut().remove_all(key)
    }

    #[deprecated(note = "use by_price().iter()")]
    pub(crate) fn iter_by_price(&self) -> PriceIter<'_> {
        PriceIter::new(OrderRefs::new(
            &self.nodes,
            self.price.iter_ids(&self.nodes),
        ))
    }

    fn link_all(&mut self, id: NodeId) {
        let id_result = self.id.insert(id, &mut self.nodes);
        let timestamp_result = self.timestamp.insert(id, &mut self.nodes);
        let trader_result = self.trader.insert(id, &mut self.nodes);
        let price_result = self.price.insert(id, &mut self.nodes);
        debug_assert!(id_result.is_ok());
        debug_assert!(timestamp_result.is_ok());
        debug_assert!(trader_result.is_ok());
        debug_assert!(price_result.is_ok());
    }

    fn unlink_all(&mut self, id: NodeId) {
        self.id.remove(id, &mut self.nodes);
        self.timestamp.remove(id, &mut self.nodes);
        self.trader.remove(id, &mut self.nodes);
        self.price.remove(id, &mut self.nodes);
    }

    fn remove_id(&mut self, id: NodeId) -> Order {
        self.unlink_all(id);
        self.nodes.remove(id.0).order
    }

    fn replace_id(&mut self, id: NodeId, replacement: Order) -> Result<Order, Conflict> {
        if self
            .id
            .find(&replacement.id, &self.nodes)
            .is_some_and(|other| other != id)
        {
            return Err(Conflict {
                index: ById::NAME,
                value: replacement,
            });
        }
        if self
            .timestamp
            .find(&replacement.timestamp, &self.nodes)
            .is_some_and(|other| other != id)
        {
            return Err(Conflict {
                index: ByTimestamp::NAME,
                value: replacement,
            });
        }

        self.unlink_all(id);
        let old = std::mem::replace(&mut self.nodes[id.0].order, replacement);
        self.link_all(id);
        self.validate_debug();
        Ok(old)
    }

    fn modify_id(&mut self, id: NodeId, f: impl FnOnce(&mut Order)) -> Result<&Order, Conflict> {
        let result = catch_unwind(AssertUnwindSafe(|| f(&mut self.nodes[id.0].order)));
        if let Err(payload) = result {
            self.remove_id(id);
            self.validate_debug();
            resume_unwind(payload);
        }

        let conflict = self
            .id
            .reconcile(id, &mut self.nodes)
            .err()
            .map(|_| ById::NAME)
            .or_else(|| {
                self.timestamp
                    .reconcile(id, &mut self.nodes)
                    .err()
                    .map(|_| ByTimestamp::NAME)
            })
            .or_else(|| {
                self.trader
                    .reconcile(id, &mut self.nodes)
                    .err()
                    .map(|_| ByTrader::NAME)
            })
            .or_else(|| {
                self.price
                    .reconcile(id, &mut self.nodes)
                    .err()
                    .map(|_| ByPrice::NAME)
            });

        if let Some(index) = conflict {
            let value = self.remove_id(id);
            self.validate_debug();
            return Err(Conflict { index, value });
        }
        self.validate_debug();
        Ok(&self.nodes[id.0].order)
    }

    fn modify_ids(&mut self, ids: Vec<NodeId>, mut f: impl FnMut(&mut Order)) -> ModifyAllResult {
        let mut result = ModifyAllResult::default();
        for id in ids {
            if !self.nodes.contains(id.0) {
                continue;
            }
            match self.modify_id(id, &mut f) {
                Ok(_) => result.modified += 1,
                Err(conflict) => result.removed.push(conflict),
            }
        }
        result
    }

    fn update_id(&mut self, id: NodeId, f: impl FnOnce(OrderUpdate<'_>)) -> &Order {
        let order = &mut self.nodes[id.0].order;
        f(OrderUpdate {
            note: &mut order.note,
            filled: &mut order.filled,
        });
        &self.nodes[id.0].order
    }

    fn update_ids(&mut self, ids: Vec<NodeId>, mut f: impl FnMut(OrderUpdate<'_>)) -> usize {
        for id in &ids {
            let order = &mut self.nodes[id.0].order;
            f(OrderUpdate {
                note: &mut order.note,
                filled: &mut order.filled,
            });
        }
        ids.len()
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        self.id.validate(&self.nodes)?;
        self.timestamp.validate(&self.nodes)?;
        self.trader.validate(&self.nodes)?;
        self.price.validate(&self.nodes)?;
        let len = self.nodes.len();
        if [
            self.id.len(),
            self.timestamp.len(),
            self.trader.len(),
            self.price.len(),
        ]
        .into_iter()
        .any(|index_len| index_len != len)
        {
            return Err("an index count differs from the arena length".to_string());
        }
        Ok(())
    }

    fn validate_debug(&self) {
        debug_assert!(self.validate().is_ok(), "{:?}", self.validate());
    }
}

struct OrderRefs<'a, I> {
    nodes: &'a Slab<OrderNode>,
    ids: I,
}

impl<'a, I> OrderRefs<'a, I> {
    fn new(nodes: &'a Slab<OrderNode>, ids: I) -> Self {
        Self { nodes, ids }
    }
}

impl<'a, I> Iterator for OrderRefs<'a, I>
where
    I: Iterator<Item = NodeId>,
{
    type Item = &'a Order;

    fn next(&mut self) -> Option<Self::Item> {
        self.ids.next().map(|id| &self.nodes[id.0].order)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.ids.size_hint()
    }
}

impl<I> DoubleEndedIterator for OrderRefs<'_, I>
where
    I: DoubleEndedIterator<Item = NodeId>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.ids.next_back().map(|id| &self.nodes[id.0].order)
    }
}

impl<I> ExactSizeIterator for OrderRefs<'_, I> where I: ExactSizeIterator<Item = NodeId> {}
impl<I> std::iter::FusedIterator for OrderRefs<'_, I> where
    I: std::iter::FusedIterator<Item = NodeId>
{
}

macro_rules! define_order_iterator {
    ($name:ident, $ids:ty) => {
        pub(crate) struct $name<'a> {
            inner: OrderRefs<'a, $ids>,
        }

        impl<'a> $name<'a> {
            fn new(inner: OrderRefs<'a, $ids>) -> Self {
                Self { inner }
            }
        }

        impl<'a> Iterator for $name<'a> {
            type Item = &'a Order;

            fn next(&mut self) -> Option<Self::Item> {
                self.inner.next()
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                self.inner.size_hint()
            }
        }

        impl std::iter::FusedIterator for $name<'_> {}
    };
}

macro_rules! impl_exact_size_iterator {
    ($name:ident) => {
        impl ExactSizeIterator for $name<'_> {}
    };
}

macro_rules! impl_double_ended_iterator {
    ($name:ident) => {
        impl DoubleEndedIterator for $name<'_> {
            fn next_back(&mut self) -> Option<Self::Item> {
                self.inner.next_back()
            }
        }
    };
}

define_order_iterator!(IdIter, HashIds<'a, OrderNode, ById, true>);
impl_exact_size_iterator!(IdIter);

define_order_iterator!(TimestampIter, OrderedIds<'a, OrderNode, ByTimestamp, true>);
impl_exact_size_iterator!(TimestampIter);
impl_double_ended_iterator!(TimestampIter);

define_order_iterator!(TraderIter, HashIds<'a, OrderNode, ByTrader, false>);
impl_exact_size_iterator!(TraderIter);

define_order_iterator!(
    TraderEqualRange,
    HashEqualIds<'a, OrderNode, ByTrader, false>
);
impl_exact_size_iterator!(TraderEqualRange);

define_order_iterator!(PriceIter, OrderedIds<'a, OrderNode, ByPrice, false>);
impl_exact_size_iterator!(PriceIter);
impl_double_ended_iterator!(PriceIter);

define_order_iterator!(
    TimestampRange,
    OrderedRangeIds<'a, OrderNode, ByTimestamp, true>
);
impl_double_ended_iterator!(TimestampRange);

define_order_iterator!(PriceRange, OrderedRangeIds<'a, OrderNode, ByPrice, false>);
impl_double_ended_iterator!(PriceRange);

pub(crate) struct IdView<'a> {
    map: &'a OrderMap,
}

impl<'a> IdView<'a> {
    pub(crate) fn get(&self, key: &u64) -> Option<&'a Order> {
        self.map
            .id
            .find(key, &self.map.nodes)
            .map(|id| &self.map.nodes[id.0].order)
    }

    pub(crate) fn contains_key(&self, key: &u64) -> bool {
        self.get(key).is_some()
    }

    pub(crate) fn iter(&self) -> IdIter<'a> {
        IdIter::new(OrderRefs::new(
            &self.map.nodes,
            self.map.id.iter_ids(&self.map.nodes),
        ))
    }
}

pub(crate) struct IdViewMut<'a> {
    map: &'a mut OrderMap,
}

impl IdViewMut<'_> {
    pub(crate) fn remove(&mut self, key: &u64) -> Option<Order> {
        let id = self.map.id.find(key, &self.map.nodes)?;
        let order = self.map.remove_id(id);
        self.map.validate_debug();
        Some(order)
    }

    pub(crate) fn replace(
        &mut self,
        key: &u64,
        replacement: Order,
    ) -> Result<Option<Order>, Conflict> {
        let Some(id) = self.map.id.find(key, &self.map.nodes) else {
            return Ok(None);
        };
        self.map.replace_id(id, replacement).map(Some)
    }

    pub(crate) fn modify(
        &mut self,
        key: &u64,
        f: impl FnOnce(&mut Order),
    ) -> Result<Option<&Order>, Conflict> {
        let Some(id) = self.map.id.find(key, &self.map.nodes) else {
            return Ok(None);
        };
        self.map.modify_id(id, f).map(Some)
    }

    pub(crate) fn update(&mut self, key: &u64, f: impl FnOnce(OrderUpdate<'_>)) -> Option<&Order> {
        let id = self.map.id.find(key, &self.map.nodes)?;
        Some(self.map.update_id(id, f))
    }
}

pub(crate) struct TimestampView<'a> {
    map: &'a OrderMap,
}

impl<'a> TimestampView<'a> {
    pub(crate) fn get(&self, key: &u64) -> Option<&'a Order> {
        self.map
            .timestamp
            .find(key, &self.map.nodes)
            .map(|id| &self.map.nodes[id.0].order)
    }

    pub(crate) fn contains_key(&self, key: &u64) -> bool {
        self.get(key).is_some()
    }

    pub(crate) fn iter(&self) -> TimestampIter<'a> {
        TimestampIter::new(OrderRefs::new(
            &self.map.nodes,
            self.map.timestamp.iter_ids(&self.map.nodes),
        ))
    }

    pub(crate) fn range<R>(&self, range: R) -> TimestampRange<'a>
    where
        R: RangeBounds<u64>,
    {
        TimestampRange::new(OrderRefs::new(
            &self.map.nodes,
            self.map.timestamp.range_iter_ids(range, &self.map.nodes),
        ))
    }
}

pub(crate) struct TimestampViewMut<'a> {
    map: &'a mut OrderMap,
}

impl TimestampViewMut<'_> {
    pub(crate) fn remove(&mut self, key: &u64) -> Option<Order> {
        let id = self.map.timestamp.find(key, &self.map.nodes)?;
        let order = self.map.remove_id(id);
        self.map.validate_debug();
        Some(order)
    }

    pub(crate) fn replace(
        &mut self,
        key: &u64,
        replacement: Order,
    ) -> Result<Option<Order>, Conflict> {
        let Some(id) = self.map.timestamp.find(key, &self.map.nodes) else {
            return Ok(None);
        };
        self.map.replace_id(id, replacement).map(Some)
    }

    pub(crate) fn modify(
        &mut self,
        key: &u64,
        f: impl FnOnce(&mut Order),
    ) -> Result<Option<&Order>, Conflict> {
        let Some(id) = self.map.timestamp.find(key, &self.map.nodes) else {
            return Ok(None);
        };
        self.map.modify_id(id, f).map(Some)
    }

    pub(crate) fn update(&mut self, key: &u64, f: impl FnOnce(OrderUpdate<'_>)) -> Option<&Order> {
        let id = self.map.timestamp.find(key, &self.map.nodes)?;
        Some(self.map.update_id(id, f))
    }
}

pub(crate) struct TraderView<'a> {
    map: &'a OrderMap,
}

impl<'a> TraderView<'a> {
    pub(crate) fn equal_range<Q>(&self, key: &Q) -> TraderEqualRange<'a>
    where
        String: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        TraderEqualRange::new(OrderRefs::new(
            &self.map.nodes,
            self.map.trader.equal_iter_ids(key, &self.map.nodes),
        ))
    }

    pub(crate) fn iter(&self) -> TraderIter<'a> {
        TraderIter::new(OrderRefs::new(
            &self.map.nodes,
            self.map.trader.iter_ids(&self.map.nodes),
        ))
    }
}

pub(crate) struct TraderViewMut<'a> {
    map: &'a mut OrderMap,
}

impl TraderViewMut<'_> {
    pub(crate) fn remove_all<Q>(&mut self, key: &Q) -> Vec<Order>
    where
        String: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        let ids = self.map.trader.equal_ids(key, &self.map.nodes);
        let orders = ids.into_iter().map(|id| self.map.remove_id(id)).collect();
        self.map.validate_debug();
        orders
    }

    pub(crate) fn modify_all<Q>(&mut self, key: &Q, f: impl FnMut(&mut Order)) -> ModifyAllResult
    where
        String: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        let ids = self.map.trader.equal_ids(key, &self.map.nodes);
        self.map.modify_ids(ids, f)
    }

    pub(crate) fn update_all<Q>(&mut self, key: &Q, f: impl FnMut(OrderUpdate<'_>)) -> usize
    where
        String: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        let ids = self.map.trader.equal_ids(key, &self.map.nodes);
        self.map.update_ids(ids, f)
    }
}

pub(crate) struct PriceView<'a> {
    map: &'a OrderMap,
}

impl<'a> PriceView<'a> {
    pub(crate) fn equal_range(&self, key: &u64) -> PriceRange<'a> {
        PriceRange::new(OrderRefs::new(
            &self.map.nodes,
            self.map.price.range_iter_ids(*key..=*key, &self.map.nodes),
        ))
    }

    pub(crate) fn iter(&self) -> PriceIter<'a> {
        PriceIter::new(OrderRefs::new(
            &self.map.nodes,
            self.map.price.iter_ids(&self.map.nodes),
        ))
    }

    pub(crate) fn range<R>(&self, range: R) -> PriceRange<'a>
    where
        R: RangeBounds<u64>,
    {
        PriceRange::new(OrderRefs::new(
            &self.map.nodes,
            self.map.price.range_iter_ids(range, &self.map.nodes),
        ))
    }
}

pub(crate) struct PriceViewMut<'a> {
    map: &'a mut OrderMap,
}

impl PriceViewMut<'_> {
    pub(crate) fn remove_all(&mut self, key: &u64) -> Vec<Order> {
        let ids = self.map.price.equal_ids(key, &self.map.nodes);
        let orders = ids.into_iter().map(|id| self.map.remove_id(id)).collect();
        self.map.validate_debug();
        orders
    }

    pub(crate) fn modify_all(&mut self, key: &u64, f: impl FnMut(&mut Order)) -> ModifyAllResult {
        let ids = self.map.price.equal_ids(key, &self.map.nodes);
        self.map.modify_ids(ids, f)
    }

    pub(crate) fn update_all(&mut self, key: &u64, f: impl FnMut(OrderUpdate<'_>)) -> usize {
        let ids = self.map.price.equal_ids(key, &self.map.nodes);
        self.map.update_ids(ids, f)
    }
}

impl IndexView for IdView<'_> {
    type Value = Order;
    type Key = u64;
    type Iter<'a>
        = IdIter<'a>
    where
        Self: 'a;

    fn len(&self) -> usize {
        self.map.id.len()
    }

    fn iter(&self) -> Self::Iter<'_> {
        IdIter::new(OrderRefs::new(
            &self.map.nodes,
            self.map.id.iter_ids(&self.map.nodes),
        ))
    }
}

impl UniqueView for IdView<'_> {
    fn get(&self, key: &Self::Key) -> Option<&Self::Value> {
        self.map
            .id
            .find(key, &self.map.nodes)
            .map(|id| &self.map.nodes[id.0].order)
    }
}

impl IndexView for IdViewMut<'_> {
    type Value = Order;
    type Key = u64;
    type Iter<'a>
        = IdIter<'a>
    where
        Self: 'a;

    fn len(&self) -> usize {
        self.map.id.len()
    }

    fn iter(&self) -> Self::Iter<'_> {
        IdIter::new(OrderRefs::new(
            &self.map.nodes,
            self.map.id.iter_ids(&self.map.nodes),
        ))
    }
}

impl UniqueView for IdViewMut<'_> {
    fn get(&self, key: &Self::Key) -> Option<&Self::Value> {
        self.map
            .id
            .find(key, &self.map.nodes)
            .map(|id| &self.map.nodes[id.0].order)
    }
}

impl UniqueViewMut for IdViewMut<'_> {
    type Conflict = Conflict;
    type Update<'a> = OrderUpdate<'a>;

    fn remove(&mut self, key: &Self::Key) -> Option<Self::Value> {
        IdViewMut::remove(self, key)
    }

    fn replace(
        &mut self,
        key: &Self::Key,
        replacement: Self::Value,
    ) -> Result<Option<Self::Value>, Self::Conflict> {
        IdViewMut::replace(self, key, replacement)
    }

    fn modify<F>(&mut self, key: &Self::Key, f: F) -> Result<Option<&Self::Value>, Self::Conflict>
    where
        F: FnOnce(&mut Self::Value),
    {
        IdViewMut::modify(self, key, f)
    }

    fn update<F>(&mut self, key: &Self::Key, f: F) -> Option<&Self::Value>
    where
        F: for<'a> FnOnce(Self::Update<'a>),
    {
        IdViewMut::update(self, key, f)
    }
}

impl IndexView for TimestampView<'_> {
    type Value = Order;
    type Key = u64;
    type Iter<'a>
        = TimestampIter<'a>
    where
        Self: 'a;

    fn len(&self) -> usize {
        self.map.timestamp.len()
    }

    fn iter(&self) -> Self::Iter<'_> {
        TimestampIter::new(OrderRefs::new(
            &self.map.nodes,
            self.map.timestamp.iter_ids(&self.map.nodes),
        ))
    }
}

impl UniqueView for TimestampView<'_> {
    fn get(&self, key: &Self::Key) -> Option<&Self::Value> {
        self.map
            .timestamp
            .find(key, &self.map.nodes)
            .map(|id| &self.map.nodes[id.0].order)
    }
}

impl OrderedView for TimestampView<'_> {
    type Range<'a>
        = TimestampRange<'a>
    where
        Self: 'a;

    fn range<R>(&self, range: R) -> Self::Range<'_>
    where
        R: RangeBounds<Self::Key>,
    {
        TimestampRange::new(OrderRefs::new(
            &self.map.nodes,
            self.map.timestamp.range_iter_ids(range, &self.map.nodes),
        ))
    }
}

impl IndexView for TimestampViewMut<'_> {
    type Value = Order;
    type Key = u64;
    type Iter<'a>
        = TimestampIter<'a>
    where
        Self: 'a;

    fn len(&self) -> usize {
        self.map.timestamp.len()
    }

    fn iter(&self) -> Self::Iter<'_> {
        TimestampIter::new(OrderRefs::new(
            &self.map.nodes,
            self.map.timestamp.iter_ids(&self.map.nodes),
        ))
    }
}

impl UniqueView for TimestampViewMut<'_> {
    fn get(&self, key: &Self::Key) -> Option<&Self::Value> {
        self.map
            .timestamp
            .find(key, &self.map.nodes)
            .map(|id| &self.map.nodes[id.0].order)
    }
}

impl OrderedView for TimestampViewMut<'_> {
    type Range<'a>
        = TimestampRange<'a>
    where
        Self: 'a;

    fn range<R>(&self, range: R) -> Self::Range<'_>
    where
        R: RangeBounds<Self::Key>,
    {
        TimestampRange::new(OrderRefs::new(
            &self.map.nodes,
            self.map.timestamp.range_iter_ids(range, &self.map.nodes),
        ))
    }
}

impl UniqueViewMut for TimestampViewMut<'_> {
    type Conflict = Conflict;
    type Update<'a> = OrderUpdate<'a>;

    fn remove(&mut self, key: &Self::Key) -> Option<Self::Value> {
        TimestampViewMut::remove(self, key)
    }

    fn replace(
        &mut self,
        key: &Self::Key,
        replacement: Self::Value,
    ) -> Result<Option<Self::Value>, Self::Conflict> {
        TimestampViewMut::replace(self, key, replacement)
    }

    fn modify<F>(&mut self, key: &Self::Key, f: F) -> Result<Option<&Self::Value>, Self::Conflict>
    where
        F: FnOnce(&mut Self::Value),
    {
        TimestampViewMut::modify(self, key, f)
    }

    fn update<F>(&mut self, key: &Self::Key, f: F) -> Option<&Self::Value>
    where
        F: for<'a> FnOnce(Self::Update<'a>),
    {
        TimestampViewMut::update(self, key, f)
    }
}

impl IndexView for TraderView<'_> {
    type Value = Order;
    type Key = String;
    type Iter<'a>
        = TraderIter<'a>
    where
        Self: 'a;

    fn len(&self) -> usize {
        self.map.trader.len()
    }

    fn iter(&self) -> Self::Iter<'_> {
        TraderIter::new(OrderRefs::new(
            &self.map.nodes,
            self.map.trader.iter_ids(&self.map.nodes),
        ))
    }
}

impl NonUniqueView for TraderView<'_> {
    type EqualRange<'a>
        = TraderEqualRange<'a>
    where
        Self: 'a;

    fn equal_range(&self, key: &Self::Key) -> Self::EqualRange<'_> {
        TraderEqualRange::new(OrderRefs::new(
            &self.map.nodes,
            self.map.trader.equal_iter_ids(key, &self.map.nodes),
        ))
    }
}

impl IndexView for TraderViewMut<'_> {
    type Value = Order;
    type Key = String;
    type Iter<'a>
        = TraderIter<'a>
    where
        Self: 'a;

    fn len(&self) -> usize {
        self.map.trader.len()
    }

    fn iter(&self) -> Self::Iter<'_> {
        TraderIter::new(OrderRefs::new(
            &self.map.nodes,
            self.map.trader.iter_ids(&self.map.nodes),
        ))
    }
}

impl NonUniqueView for TraderViewMut<'_> {
    type EqualRange<'a>
        = TraderEqualRange<'a>
    where
        Self: 'a;

    fn equal_range(&self, key: &Self::Key) -> Self::EqualRange<'_> {
        TraderEqualRange::new(OrderRefs::new(
            &self.map.nodes,
            self.map.trader.equal_iter_ids(key, &self.map.nodes),
        ))
    }
}

impl NonUniqueViewMut for TraderViewMut<'_> {
    type ModifyAllResult = ModifyAllResult;
    type Update<'a> = OrderUpdate<'a>;

    fn remove_all(&mut self, key: &Self::Key) -> Vec<Self::Value> {
        TraderViewMut::remove_all(self, key)
    }

    fn modify_all<F>(&mut self, key: &Self::Key, f: F) -> Self::ModifyAllResult
    where
        F: FnMut(&mut Self::Value),
    {
        TraderViewMut::modify_all(self, key, f)
    }

    fn update_all<F>(&mut self, key: &Self::Key, f: F) -> usize
    where
        F: for<'a> FnMut(Self::Update<'a>),
    {
        TraderViewMut::update_all(self, key, f)
    }
}

impl IndexView for PriceView<'_> {
    type Value = Order;
    type Key = u64;
    type Iter<'a>
        = PriceIter<'a>
    where
        Self: 'a;

    fn len(&self) -> usize {
        self.map.price.len()
    }

    fn iter(&self) -> Self::Iter<'_> {
        PriceIter::new(OrderRefs::new(
            &self.map.nodes,
            self.map.price.iter_ids(&self.map.nodes),
        ))
    }
}

impl NonUniqueView for PriceView<'_> {
    type EqualRange<'a>
        = PriceRange<'a>
    where
        Self: 'a;

    fn equal_range(&self, key: &Self::Key) -> Self::EqualRange<'_> {
        PriceRange::new(OrderRefs::new(
            &self.map.nodes,
            self.map.price.range_iter_ids(*key..=*key, &self.map.nodes),
        ))
    }
}

impl OrderedView for PriceView<'_> {
    type Range<'a>
        = PriceRange<'a>
    where
        Self: 'a;

    fn range<R>(&self, range: R) -> Self::Range<'_>
    where
        R: RangeBounds<Self::Key>,
    {
        PriceRange::new(OrderRefs::new(
            &self.map.nodes,
            self.map.price.range_iter_ids(range, &self.map.nodes),
        ))
    }
}

impl IndexView for PriceViewMut<'_> {
    type Value = Order;
    type Key = u64;
    type Iter<'a>
        = PriceIter<'a>
    where
        Self: 'a;

    fn len(&self) -> usize {
        self.map.price.len()
    }

    fn iter(&self) -> Self::Iter<'_> {
        PriceIter::new(OrderRefs::new(
            &self.map.nodes,
            self.map.price.iter_ids(&self.map.nodes),
        ))
    }
}

impl NonUniqueView for PriceViewMut<'_> {
    type EqualRange<'a>
        = PriceRange<'a>
    where
        Self: 'a;

    fn equal_range(&self, key: &Self::Key) -> Self::EqualRange<'_> {
        PriceRange::new(OrderRefs::new(
            &self.map.nodes,
            self.map.price.range_iter_ids(*key..=*key, &self.map.nodes),
        ))
    }
}

impl OrderedView for PriceViewMut<'_> {
    type Range<'a>
        = PriceRange<'a>
    where
        Self: 'a;

    fn range<R>(&self, range: R) -> Self::Range<'_>
    where
        R: RangeBounds<Self::Key>,
    {
        PriceRange::new(OrderRefs::new(
            &self.map.nodes,
            self.map.price.range_iter_ids(range, &self.map.nodes),
        ))
    }
}

impl NonUniqueViewMut for PriceViewMut<'_> {
    type ModifyAllResult = ModifyAllResult;
    type Update<'a> = OrderUpdate<'a>;

    fn remove_all(&mut self, key: &Self::Key) -> Vec<Self::Value> {
        PriceViewMut::remove_all(self, key)
    }

    fn modify_all<F>(&mut self, key: &Self::Key, f: F) -> Self::ModifyAllResult
    where
        F: FnMut(&mut Self::Value),
    {
        PriceViewMut::modify_all(self, key, f)
    }

    fn update_all<F>(&mut self, key: &Self::Key, f: F) -> usize
    where
        F: for<'a> FnMut(Self::Update<'a>),
    {
        PriceViewMut::update_all(self, key, f)
    }
}
