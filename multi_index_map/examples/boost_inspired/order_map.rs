use crate::index::{
    HashLink, HashSpec, HashedIndex, NodeId, NodeValue, OrderedIndex, OrderedLink, OrderedSpec,
};
use slab::Slab;
use std::borrow::Borrow;
use std::fmt;
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

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Conflict {
    pub(crate) index: &'static str,
    pub(crate) value: Order,
}

impl fmt::Display for Conflict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unique index '{}' rejected the value", self.index)
    }
}

pub(crate) struct OrderUpdate<'a> {
    pub(crate) note: &'a mut String,
    pub(crate) filled: &'a mut bool,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct ModifyAllResult {
    pub(crate) modified: usize,
    pub(crate) removed: Vec<Conflict>,
}

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
    type Key = str;

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
            .map_or(false, |other| other != id)
        {
            return Err(Conflict {
                index: ById::NAME,
                value: replacement,
            });
        }
        if self
            .timestamp
            .find(&replacement.timestamp, &self.nodes)
            .map_or(false, |other| other != id)
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

    pub(crate) fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = &'a Order> + std::iter::FusedIterator + 'a {
        OrderRefs::new(&self.map.nodes, self.map.id.iter_ids(&self.map.nodes))
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

    pub(crate) fn iter(
        &self,
    ) -> impl DoubleEndedIterator<Item = &'a Order> + ExactSizeIterator + std::iter::FusedIterator + 'a
    {
        OrderRefs::new(
            &self.map.nodes,
            self.map.timestamp.iter_ids(&self.map.nodes),
        )
    }

    pub(crate) fn range<R>(
        &self,
        range: R,
    ) -> impl DoubleEndedIterator<Item = &'a Order> + std::iter::FusedIterator + 'a
    where
        R: RangeBounds<u64>,
    {
        OrderRefs::new(
            &self.map.nodes,
            self.map.timestamp.range_iter_ids(range, &self.map.nodes),
        )
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
    pub(crate) fn equal_range<Q>(
        &self,
        key: &Q,
    ) -> impl ExactSizeIterator<Item = &'a Order> + std::iter::FusedIterator + 'a
    where
        str: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        OrderRefs::new(
            &self.map.nodes,
            self.map.trader.equal_iter_ids(key, &self.map.nodes),
        )
    }

    pub(crate) fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = &'a Order> + std::iter::FusedIterator + 'a {
        OrderRefs::new(&self.map.nodes, self.map.trader.iter_ids(&self.map.nodes))
    }
}

pub(crate) struct TraderViewMut<'a> {
    map: &'a mut OrderMap,
}

impl TraderViewMut<'_> {
    pub(crate) fn remove_all<Q>(&mut self, key: &Q) -> Vec<Order>
    where
        str: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        let ids = self.map.trader.equal_ids(key, &self.map.nodes);
        let orders = ids.into_iter().map(|id| self.map.remove_id(id)).collect();
        self.map.validate_debug();
        orders
    }

    pub(crate) fn modify_all<Q>(&mut self, key: &Q, f: impl FnMut(&mut Order)) -> ModifyAllResult
    where
        str: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        let ids = self.map.trader.equal_ids(key, &self.map.nodes);
        self.map.modify_ids(ids, f)
    }

    pub(crate) fn update_all<Q>(&mut self, key: &Q, f: impl FnMut(OrderUpdate<'_>)) -> usize
    where
        str: Borrow<Q>,
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
    pub(crate) fn equal_range(
        &self,
        key: &u64,
    ) -> impl DoubleEndedIterator<Item = &'a Order> + std::iter::FusedIterator + 'a {
        OrderRefs::new(
            &self.map.nodes,
            self.map.price.range_iter_ids(*key..=*key, &self.map.nodes),
        )
    }

    pub(crate) fn iter(
        &self,
    ) -> impl DoubleEndedIterator<Item = &'a Order> + ExactSizeIterator + std::iter::FusedIterator + 'a
    {
        OrderRefs::new(&self.map.nodes, self.map.price.iter_ids(&self.map.nodes))
    }

    pub(crate) fn range<R>(
        &self,
        range: R,
    ) -> impl DoubleEndedIterator<Item = &'a Order> + std::iter::FusedIterator + 'a
    where
        R: RangeBounds<u64>,
    {
        OrderRefs::new(
            &self.map.nodes,
            self.map.price.range_iter_ids(range, &self.map.nodes),
        )
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
