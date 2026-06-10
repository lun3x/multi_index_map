use multi_index_map::__private::{
    HashEqualIds, HashIds, HashLink, HashedIndex, IndexSpec, NodeId, NodeValue, OrderedIds,
    OrderedIndex, OrderedLink, OrderedRangeIds, Slab,
};
use multi_index_map::{Conflict as GenericConflict, ModifyAllResult as GenericModifyAllResult};
use multi_index_map::{
    IndexView, IndexViewMut, MultiIndexSelector, NonUniqueView, NonUniqueViewMut, OrderedView,
    UniqueView, UniqueViewMut,
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
    trader_timestamp_link: OrderedLink,
}

impl OrderNode {
    fn new(order: Order) -> Self {
        Self {
            order,
            id_link: HashLink::default(),
            timestamp_link: OrderedLink::default(),
            trader_link: HashLink::default(),
            price_link: OrderedLink::default(),
            trader_timestamp_link: OrderedLink::default(),
        }
    }
}

impl NodeValue for OrderNode {
    type Value = Order;

    fn value(&self) -> &Self::Value {
        &self.order
    }
}

#[derive(MultiIndexSelector)]
#[multi_index(hashed_unique)]
pub(crate) struct ById;
#[derive(MultiIndexSelector)]
#[multi_index(ordered_unique)]
pub(crate) struct ByTimestamp;
#[derive(MultiIndexSelector)]
#[multi_index(hashed_non_unique)]
pub(crate) struct ByTrader;
#[derive(MultiIndexSelector)]
#[multi_index(ordered_non_unique)]
pub(crate) struct ByPrice;
#[derive(MultiIndexSelector)]
#[multi_index(ordered_non_unique)]
pub(crate) struct ByTraderTimestamp;

impl IndexSpec<OrderNode> for ById {
    type Key<'a> = &'a u64;
    type Link = HashLink;

    const NAME: &'static str = "id";

    fn key(value: &Order) -> Self::Key<'_> {
        &value.id
    }

    fn link(node: &OrderNode) -> &HashLink {
        &node.id_link
    }

    fn link_mut(node: &mut OrderNode) -> &mut HashLink {
        &mut node.id_link
    }
}

impl IndexSpec<OrderNode> for ByTimestamp {
    type Key<'a> = &'a u64;
    type Link = OrderedLink;

    const NAME: &'static str = "timestamp";

    fn key(value: &Order) -> Self::Key<'_> {
        &value.timestamp
    }

    fn link(node: &OrderNode) -> &OrderedLink {
        &node.timestamp_link
    }

    fn link_mut(node: &mut OrderNode) -> &mut OrderedLink {
        &mut node.timestamp_link
    }
}

impl IndexSpec<OrderNode> for ByTrader {
    type Key<'a> = &'a String;
    type Link = HashLink;

    const NAME: &'static str = "trader";

    fn key(value: &Order) -> Self::Key<'_> {
        &value.trader
    }

    fn link(node: &OrderNode) -> &HashLink {
        &node.trader_link
    }

    fn link_mut(node: &mut OrderNode) -> &mut HashLink {
        &mut node.trader_link
    }
}

impl IndexSpec<OrderNode> for ByPrice {
    type Key<'a> = &'a u64;
    type Link = OrderedLink;

    const NAME: &'static str = "price";

    fn key(value: &Order) -> Self::Key<'_> {
        &value.price
    }

    fn link(node: &OrderNode) -> &OrderedLink {
        &node.price_link
    }

    fn link_mut(node: &mut OrderNode) -> &mut OrderedLink {
        &mut node.price_link
    }
}

impl IndexSpec<OrderNode> for ByTraderTimestamp {
    type Key<'a> = (&'a String, &'a u64);
    type Link = OrderedLink;

    const NAME: &'static str = "ByTraderTimestamp";

    fn key(value: &Order) -> Self::Key<'_> {
        (&value.trader, &value.timestamp)
    }

    fn link(node: &OrderNode) -> &OrderedLink {
        &node.trader_timestamp_link
    }

    fn link_mut(node: &mut OrderNode) -> &mut OrderedLink {
        &mut node.trader_timestamp_link
    }
}

type IdIndex = HashedIndex<ById, true>;
type TimestampIndex = OrderedIndex<ByTimestamp, true>;
type TraderIndex = HashedIndex<ByTrader, false>;
type PriceIndex = OrderedIndex<ByPrice, false>;
type TraderTimestampIndex = OrderedIndex<ByTraderTimestamp, false>;

pub(crate) trait OrderMapIndex: MultiIndexSelector {
    type View<'a>
    where
        Self: 'a;

    type ViewMut<'a>
    where
        Self: 'a;

    fn view(map: &OrderMap) -> Self::View<'_>;
    fn view_mut(map: &mut OrderMap) -> Self::ViewMut<'_>;
}

impl OrderMapIndex for ById {
    type View<'a> = IdView<'a>;
    type ViewMut<'a> = IdViewMut<'a>;

    fn view(map: &OrderMap) -> Self::View<'_> {
        IdView { map }
    }

    fn view_mut(map: &mut OrderMap) -> Self::ViewMut<'_> {
        IdViewMut { map }
    }
}

impl OrderMapIndex for ByTimestamp {
    type View<'a> = TimestampView<'a>;
    type ViewMut<'a> = TimestampViewMut<'a>;

    fn view(map: &OrderMap) -> Self::View<'_> {
        TimestampView { map }
    }

    fn view_mut(map: &mut OrderMap) -> Self::ViewMut<'_> {
        TimestampViewMut { map }
    }
}

impl OrderMapIndex for ByTrader {
    type View<'a> = TraderView<'a>;
    type ViewMut<'a> = TraderViewMut<'a>;

    fn view(map: &OrderMap) -> Self::View<'_> {
        TraderView { map }
    }

    fn view_mut(map: &mut OrderMap) -> Self::ViewMut<'_> {
        TraderViewMut { map }
    }
}

impl OrderMapIndex for ByPrice {
    type View<'a> = PriceView<'a>;
    type ViewMut<'a> = PriceViewMut<'a>;

    fn view(map: &OrderMap) -> Self::View<'_> {
        PriceView { map }
    }

    fn view_mut(map: &mut OrderMap) -> Self::ViewMut<'_> {
        PriceViewMut { map }
    }
}

impl OrderMapIndex for ByTraderTimestamp {
    type View<'a> = TraderTimestampView<'a>;
    type ViewMut<'a> = TraderTimestampViewMut<'a>;

    fn view(map: &OrderMap) -> Self::View<'_> {
        TraderTimestampView { map }
    }

    fn view_mut(map: &mut OrderMap) -> Self::ViewMut<'_> {
        TraderTimestampViewMut { map }
    }
}

#[derive(Default)]
pub(crate) struct OrderMap {
    inner: OrderMapInner,
}

#[derive(Default)]
struct OrderMapInner {
    nodes: Slab<OrderNode>,
    id: IdIndex,
    timestamp: TimestampIndex,
    trader: TraderIndex,
    price: PriceIndex,
    trader_timestamp: TraderTimestampIndex,
}

impl OrderMap {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn len(&self) -> usize {
        self.inner.nodes.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.inner.nodes.is_empty()
    }

    pub(crate) fn insert(&mut self, order: Order) -> Result<&Order, Conflict> {
        self.inner.insert(order)
    }

    pub(crate) fn clear(&mut self) {
        self.inner.clear();
    }

    pub(crate) fn by<I: OrderMapIndex>(&self) -> I::View<'_> {
        I::view(self)
    }

    pub(crate) fn by_mut<I: OrderMapIndex>(&mut self) -> I::ViewMut<'_> {
        I::view_mut(self)
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        self.inner.validate()
    }
}

impl OrderMapInner {
    fn insert(&mut self, order: Order) -> Result<&Order, Conflict> {
        if self.id.find(&order.id, &self.nodes).is_some() {
            return Err(Conflict {
                index: <ById as MultiIndexSelector>::NAME,
                value: order,
            });
        }
        if self.timestamp.find(&order.timestamp, &self.nodes).is_some() {
            return Err(Conflict {
                index: <ByTimestamp as MultiIndexSelector>::NAME,
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

    fn clear(&mut self) {
        let ids: Vec<_> = self.nodes.iter().map(|(id, _)| NodeId(id)).collect();
        for id in ids {
            self.remove_id(id);
        }
        self.validate_debug();
    }

    fn update_fields_for_ids(&mut self, mut ids: Vec<NodeId>) -> Vec<(&mut String, &mut bool)> {
        ids.sort_unstable_by_key(|id| id.0);
        assert!(
            !ids.windows(2).any(|pair| pair[0] == pair[1]),
            "compatibility selector received a duplicate arena node"
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
            "compatibility selector targeted a missing arena node"
        );
        fields
    }

    fn order_refs_for_ids(&self, ids: &[NodeId]) -> Vec<&Order> {
        ids.iter()
            .map(|id| {
                &self
                    .nodes
                    .get(id.0)
                    .expect("compatibility selector targeted a missing arena node")
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
}

impl OrderMap {
    #[deprecated(note = "use by_id().get(key)")]
    pub(crate) fn get_by_id<Q>(&self, key: &Q) -> Option<&Order>
    where
        u64: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.inner
            .id
            .find(key, &self.inner.nodes)
            .map(|id| &self.inner.nodes[id.0].order)
    }

    #[deprecated(note = "use by_id_mut().update(key, ...)")]
    pub(crate) fn get_mut_by_id(&mut self, key: &u64) -> Option<(&mut String, &mut bool)> {
        let id = self.inner.id.find(key, &self.inner.nodes)?;
        self.inner
            .update_fields_for_ids(vec![id])
            .into_iter()
            .next()
    }

    #[deprecated(note = "use by_id_mut().modify(key, ...)")]
    pub(crate) fn modify_by_id(&mut self, key: &u64, f: impl FnOnce(&mut Order)) -> Option<&Order> {
        let id = self.inner.id.find(key, &self.inner.nodes)?;
        match self.inner.modify_id(id, f) {
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
        let id = self.inner.id.find(key, &self.inner.nodes)?;
        Some(
            self.inner
                .update_id(id, |fields| f(fields.note, fields.filled)),
        )
    }

    #[deprecated(note = "use by_id_mut().remove(key)")]
    pub(crate) fn remove_by_id(&mut self, key: &u64) -> Option<Order> {
        self.by_mut::<ById>().remove(key)
    }

    #[deprecated(note = "use by_id().iter()")]
    pub(crate) fn iter_by_id(&self) -> IdIter<'_> {
        IdIter::new(OrderRefs::new(
            &self.inner.nodes,
            self.inner.id.iter_ids(&self.inner.nodes),
        ))
    }

    #[deprecated(note = "use by_timestamp().get(key)")]
    pub(crate) fn get_by_timestamp<Q>(&self, key: &Q) -> Option<&Order>
    where
        u64: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.inner
            .timestamp
            .find(key, &self.inner.nodes)
            .map(|id| &self.inner.nodes[id.0].order)
    }

    #[deprecated(note = "use by_timestamp_mut().update(key, ...)")]
    pub(crate) fn get_mut_by_timestamp(&mut self, key: &u64) -> Option<(&mut String, &mut bool)> {
        let id = self.inner.timestamp.find(key, &self.inner.nodes)?;
        self.inner
            .update_fields_for_ids(vec![id])
            .into_iter()
            .next()
    }

    #[deprecated(note = "use by_timestamp_mut().modify(key, ...)")]
    pub(crate) fn modify_by_timestamp(
        &mut self,
        key: &u64,
        f: impl FnOnce(&mut Order),
    ) -> Option<&Order> {
        let id = self.inner.timestamp.find(key, &self.inner.nodes)?;
        match self.inner.modify_id(id, f) {
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
        let id = self.inner.timestamp.find(key, &self.inner.nodes)?;
        Some(
            self.inner
                .update_id(id, |fields| f(fields.note, fields.filled)),
        )
    }

    #[deprecated(note = "use by_timestamp_mut().remove(key)")]
    pub(crate) fn remove_by_timestamp(&mut self, key: &u64) -> Option<Order> {
        self.by_mut::<ByTimestamp>().remove(key)
    }

    #[deprecated(note = "use by_timestamp().iter()")]
    pub(crate) fn iter_by_timestamp(&self) -> TimestampIter<'_> {
        TimestampIter::new(OrderRefs::new(
            &self.inner.nodes,
            self.inner.timestamp.iter_ids(&self.inner.nodes),
        ))
    }

    #[deprecated(note = "use by_trader().equal_range(key)")]
    pub(crate) fn get_by_trader<Q>(&self, key: &Q) -> Vec<&Order>
    where
        String: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.by::<ByTrader>().equal_range(key).collect()
    }

    #[deprecated(note = "use by_trader_mut().update_all(key, ...)")]
    pub(crate) fn get_mut_by_trader(&mut self, key: &String) -> Vec<(&mut String, &mut bool)> {
        let ids = self.inner.trader.equal_ids(key, &self.inner.nodes);
        self.inner.update_fields_for_ids(ids)
    }

    #[deprecated(note = "use by_trader_mut().modify_all(key, ...)")]
    pub(crate) fn modify_by_trader(
        &mut self,
        key: &String,
        f: impl FnMut(&mut Order),
    ) -> Vec<&Order> {
        let ids = self.inner.trader.equal_ids(key, &self.inner.nodes);
        let result = self.by_mut::<ByTrader>().modify_all(key, f);
        OrderMapInner::panic_on_modify_conflicts(result);
        self.inner.order_refs_for_ids(&ids)
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
        let ids = self.inner.trader.equal_ids(key, &self.inner.nodes);
        self.by_mut::<ByTrader>()
            .update_all(key, |fields| f(fields.note, fields.filled));
        self.inner.order_refs_for_ids(&ids)
    }

    #[deprecated(note = "use by_trader_mut().remove_all(key)")]
    pub(crate) fn remove_by_trader(&mut self, key: &String) -> Vec<Order> {
        self.by_mut::<ByTrader>().remove_all(key)
    }

    #[deprecated(note = "use by_trader().iter()")]
    pub(crate) fn iter_by_trader(&self) -> TraderIter<'_> {
        TraderIter::new(OrderRefs::new(
            &self.inner.nodes,
            self.inner.trader.iter_ids(&self.inner.nodes),
        ))
    }

    #[deprecated(note = "use by_price().equal_range(key)")]
    pub(crate) fn get_by_price<Q>(&self, key: &Q) -> Vec<&Order>
    where
        u64: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.inner
            .price
            .equal_ids(key, &self.inner.nodes)
            .into_iter()
            .map(|id| &self.inner.nodes[id.0].order)
            .collect()
    }

    #[deprecated(note = "use by_price_mut().update_all(key, ...)")]
    pub(crate) fn get_mut_by_price(&mut self, key: &u64) -> Vec<(&mut String, &mut bool)> {
        let ids = self.inner.price.equal_ids(key, &self.inner.nodes);
        self.inner.update_fields_for_ids(ids)
    }

    #[deprecated(note = "use by_price_mut().modify_all(key, ...)")]
    pub(crate) fn modify_by_price(&mut self, key: &u64, f: impl FnMut(&mut Order)) -> Vec<&Order> {
        let ids = self.inner.price.equal_ids(key, &self.inner.nodes);
        let result = self.by_mut::<ByPrice>().modify_all(key, f);
        OrderMapInner::panic_on_modify_conflicts(result);
        self.inner.order_refs_for_ids(&ids)
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
        let ids = self.inner.price.equal_ids(key, &self.inner.nodes);
        for id in &ids {
            self.inner
                .update_id(*id, |fields| f(fields.note, fields.filled));
        }
        self.inner.order_refs_for_ids(&ids)
    }

    #[deprecated(note = "use by_price_mut().remove_all(key)")]
    pub(crate) fn remove_by_price(&mut self, key: &u64) -> Vec<Order> {
        self.by_mut::<ByPrice>().remove_all(key)
    }

    #[deprecated(note = "use by_price().iter()")]
    pub(crate) fn iter_by_price(&self) -> PriceIter<'_> {
        PriceIter::new(OrderRefs::new(
            &self.inner.nodes,
            self.inner.price.iter_ids(&self.inner.nodes),
        ))
    }
}

impl OrderMapInner {
    fn link_all(&mut self, id: NodeId) {
        let id_result = self.id.insert(id, &mut self.nodes);
        let timestamp_result = self.timestamp.insert(id, &mut self.nodes);
        let trader_result = self.trader.insert(id, &mut self.nodes);
        let price_result = self.price.insert(id, &mut self.nodes);
        let trader_timestamp_result = self.trader_timestamp.insert(id, &mut self.nodes);
        debug_assert!(id_result.is_ok());
        debug_assert!(timestamp_result.is_ok());
        debug_assert!(trader_result.is_ok());
        debug_assert!(price_result.is_ok());
        debug_assert!(trader_timestamp_result.is_ok());
    }

    fn unlink_all(&mut self, id: NodeId) {
        self.id.remove(id, &mut self.nodes);
        self.timestamp.remove(id, &mut self.nodes);
        self.trader.remove(id, &mut self.nodes);
        self.price.remove(id, &mut self.nodes);
        self.trader_timestamp.remove(id, &mut self.nodes);
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
                index: <ById as MultiIndexSelector>::NAME,
                value: replacement,
            });
        }
        if self
            .timestamp
            .find(&replacement.timestamp, &self.nodes)
            .is_some_and(|other| other != id)
        {
            return Err(Conflict {
                index: <ByTimestamp as MultiIndexSelector>::NAME,
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
            .map(|_| <ById as MultiIndexSelector>::NAME)
            .or_else(|| {
                self.timestamp
                    .reconcile(id, &mut self.nodes)
                    .err()
                    .map(|_| <ByTimestamp as MultiIndexSelector>::NAME)
            })
            .or_else(|| {
                self.trader
                    .reconcile(id, &mut self.nodes)
                    .err()
                    .map(|_| <ByTrader as MultiIndexSelector>::NAME)
            })
            .or_else(|| {
                self.price
                    .reconcile(id, &mut self.nodes)
                    .err()
                    .map(|_| <ByPrice as MultiIndexSelector>::NAME)
            })
            .or_else(|| {
                self.trader_timestamp
                    .reconcile(id, &mut self.nodes)
                    .err()
                    .map(|_| <ByTraderTimestamp as MultiIndexSelector>::NAME)
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

    fn validate(&self) -> Result<(), String> {
        self.id.validate(&self.nodes)?;
        self.timestamp.validate(&self.nodes)?;
        self.trader.validate(&self.nodes)?;
        self.price.validate(&self.nodes)?;
        self.trader_timestamp.validate(&self.nodes)?;
        let len = self.nodes.len();
        if [
            self.id.len(),
            self.timestamp.len(),
            self.trader.len(),
            self.price.len(),
            self.trader_timestamp.len(),
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

define_order_iterator!(
    TraderTimestampIter,
    OrderedIds<'a, OrderNode, ByTraderTimestamp, false>
);
impl_exact_size_iterator!(TraderTimestampIter);
impl_double_ended_iterator!(TraderTimestampIter);

define_order_iterator!(
    TraderTimestampRange,
    OrderedRangeIds<'a, OrderNode, ByTraderTimestamp, false>
);
impl_double_ended_iterator!(TraderTimestampRange);

pub(crate) struct IdView<'a> {
    map: &'a OrderMap,
}

impl<'a> IdView<'a> {
    pub(crate) fn get(&self, key: &u64) -> Option<&'a Order> {
        self.map
            .inner
            .id
            .find(key, &self.map.inner.nodes)
            .map(|id| &self.map.inner.nodes[id.0].order)
    }

    pub(crate) fn contains_key(&self, key: &u64) -> bool {
        self.get(key).is_some()
    }

    pub(crate) fn iter(&self) -> IdIter<'a> {
        IdIter::new(OrderRefs::new(
            &self.map.inner.nodes,
            self.map.inner.id.iter_ids(&self.map.inner.nodes),
        ))
    }
}

pub(crate) struct IdViewMut<'a> {
    map: &'a mut OrderMap,
}

impl IdViewMut<'_> {
    pub(crate) fn update_each(&mut self, f: impl FnMut(OrderUpdate<'_>)) -> usize {
        let ids = self.map.inner.id.iter_ids(&self.map.inner.nodes).collect();
        self.map.inner.update_ids(ids, f)
    }

    pub(crate) fn remove(&mut self, key: &u64) -> Option<Order> {
        let id = self.map.inner.id.find(key, &self.map.inner.nodes)?;
        let order = self.map.inner.remove_id(id);
        self.map.inner.validate_debug();
        Some(order)
    }

    pub(crate) fn replace(
        &mut self,
        key: &u64,
        replacement: Order,
    ) -> Result<Option<Order>, Conflict> {
        let Some(id) = self.map.inner.id.find(key, &self.map.inner.nodes) else {
            return Ok(None);
        };
        self.map.inner.replace_id(id, replacement).map(Some)
    }

    pub(crate) fn modify(
        &mut self,
        key: &u64,
        f: impl FnOnce(&mut Order),
    ) -> Result<Option<&Order>, Conflict> {
        let Some(id) = self.map.inner.id.find(key, &self.map.inner.nodes) else {
            return Ok(None);
        };
        self.map.inner.modify_id(id, f).map(Some)
    }

    pub(crate) fn update(&mut self, key: &u64, f: impl FnOnce(OrderUpdate<'_>)) -> Option<&Order> {
        let id = self.map.inner.id.find(key, &self.map.inner.nodes)?;
        Some(self.map.inner.update_id(id, f))
    }
}

pub(crate) struct TimestampView<'a> {
    map: &'a OrderMap,
}

impl<'a> TimestampView<'a> {
    pub(crate) fn get(&self, key: &u64) -> Option<&'a Order> {
        self.map
            .inner
            .timestamp
            .find(key, &self.map.inner.nodes)
            .map(|id| &self.map.inner.nodes[id.0].order)
    }

    pub(crate) fn contains_key(&self, key: &u64) -> bool {
        self.get(key).is_some()
    }

    pub(crate) fn iter(&self) -> TimestampIter<'a> {
        TimestampIter::new(OrderRefs::new(
            &self.map.inner.nodes,
            self.map.inner.timestamp.iter_ids(&self.map.inner.nodes),
        ))
    }

    pub(crate) fn range<R>(&self, range: R) -> TimestampRange<'a>
    where
        R: RangeBounds<u64>,
    {
        TimestampRange::new(OrderRefs::new(
            &self.map.inner.nodes,
            self.map
                .inner
                .timestamp
                .range_iter_ids(range, &self.map.inner.nodes),
        ))
    }
}

pub(crate) struct TimestampViewMut<'a> {
    map: &'a mut OrderMap,
}

impl TimestampViewMut<'_> {
    pub(crate) fn update_each(&mut self, f: impl FnMut(OrderUpdate<'_>)) -> usize {
        let ids = self
            .map
            .inner
            .timestamp
            .iter_ids(&self.map.inner.nodes)
            .collect();
        self.map.inner.update_ids(ids, f)
    }

    pub(crate) fn remove(&mut self, key: &u64) -> Option<Order> {
        let id = self.map.inner.timestamp.find(key, &self.map.inner.nodes)?;
        let order = self.map.inner.remove_id(id);
        self.map.inner.validate_debug();
        Some(order)
    }

    pub(crate) fn replace(
        &mut self,
        key: &u64,
        replacement: Order,
    ) -> Result<Option<Order>, Conflict> {
        let Some(id) = self.map.inner.timestamp.find(key, &self.map.inner.nodes) else {
            return Ok(None);
        };
        self.map.inner.replace_id(id, replacement).map(Some)
    }

    pub(crate) fn modify(
        &mut self,
        key: &u64,
        f: impl FnOnce(&mut Order),
    ) -> Result<Option<&Order>, Conflict> {
        let Some(id) = self.map.inner.timestamp.find(key, &self.map.inner.nodes) else {
            return Ok(None);
        };
        self.map.inner.modify_id(id, f).map(Some)
    }

    pub(crate) fn update(&mut self, key: &u64, f: impl FnOnce(OrderUpdate<'_>)) -> Option<&Order> {
        let id = self.map.inner.timestamp.find(key, &self.map.inner.nodes)?;
        Some(self.map.inner.update_id(id, f))
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
            &self.map.inner.nodes,
            self.map
                .inner
                .trader
                .equal_iter_ids(key, &self.map.inner.nodes),
        ))
    }

    pub(crate) fn iter(&self) -> TraderIter<'a> {
        TraderIter::new(OrderRefs::new(
            &self.map.inner.nodes,
            self.map.inner.trader.iter_ids(&self.map.inner.nodes),
        ))
    }
}

pub(crate) struct TraderViewMut<'a> {
    map: &'a mut OrderMap,
}

impl TraderViewMut<'_> {
    pub(crate) fn update_each(&mut self, f: impl FnMut(OrderUpdate<'_>)) -> usize {
        let ids = self
            .map
            .inner
            .trader
            .iter_ids(&self.map.inner.nodes)
            .collect();
        self.map.inner.update_ids(ids, f)
    }

    pub(crate) fn remove_all<Q>(&mut self, key: &Q) -> Vec<Order>
    where
        String: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        let ids = self.map.inner.trader.equal_ids(key, &self.map.inner.nodes);
        let orders = ids
            .into_iter()
            .map(|id| self.map.inner.remove_id(id))
            .collect();
        self.map.inner.validate_debug();
        orders
    }

    pub(crate) fn modify_all<Q>(&mut self, key: &Q, f: impl FnMut(&mut Order)) -> ModifyAllResult
    where
        String: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        let ids = self.map.inner.trader.equal_ids(key, &self.map.inner.nodes);
        self.map.inner.modify_ids(ids, f)
    }

    pub(crate) fn update_all<Q>(&mut self, key: &Q, f: impl FnMut(OrderUpdate<'_>)) -> usize
    where
        String: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        let ids = self.map.inner.trader.equal_ids(key, &self.map.inner.nodes);
        self.map.inner.update_ids(ids, f)
    }
}

pub(crate) struct PriceView<'a> {
    map: &'a OrderMap,
}

impl<'a> PriceView<'a> {
    pub(crate) fn equal_range(&self, key: &u64) -> PriceRange<'a> {
        PriceRange::new(OrderRefs::new(
            &self.map.inner.nodes,
            self.map
                .inner
                .price
                .range_iter_ids(*key..=*key, &self.map.inner.nodes),
        ))
    }

    pub(crate) fn iter(&self) -> PriceIter<'a> {
        PriceIter::new(OrderRefs::new(
            &self.map.inner.nodes,
            self.map.inner.price.iter_ids(&self.map.inner.nodes),
        ))
    }

    pub(crate) fn range<R>(&self, range: R) -> PriceRange<'a>
    where
        R: RangeBounds<u64>,
    {
        PriceRange::new(OrderRefs::new(
            &self.map.inner.nodes,
            self.map
                .inner
                .price
                .range_iter_ids(range, &self.map.inner.nodes),
        ))
    }
}

pub(crate) struct PriceViewMut<'a> {
    map: &'a mut OrderMap,
}

impl PriceViewMut<'_> {
    pub(crate) fn update_each(&mut self, f: impl FnMut(OrderUpdate<'_>)) -> usize {
        let ids = self
            .map
            .inner
            .price
            .iter_ids(&self.map.inner.nodes)
            .collect();
        self.map.inner.update_ids(ids, f)
    }

    pub(crate) fn remove_all(&mut self, key: &u64) -> Vec<Order> {
        let ids = self.map.inner.price.equal_ids(key, &self.map.inner.nodes);
        let orders = ids
            .into_iter()
            .map(|id| self.map.inner.remove_id(id))
            .collect();
        self.map.inner.validate_debug();
        orders
    }

    pub(crate) fn modify_all(&mut self, key: &u64, f: impl FnMut(&mut Order)) -> ModifyAllResult {
        let ids = self.map.inner.price.equal_ids(key, &self.map.inner.nodes);
        self.map.inner.modify_ids(ids, f)
    }

    pub(crate) fn update_all(&mut self, key: &u64, f: impl FnMut(OrderUpdate<'_>)) -> usize {
        let ids = self.map.inner.price.equal_ids(key, &self.map.inner.nodes);
        self.map.inner.update_ids(ids, f)
    }
}

pub(crate) struct TraderTimestampView<'a> {
    map: &'a OrderMap,
}

impl<'a> TraderTimestampView<'a> {
    pub(crate) fn equal_range<'query, Q0, Q1>(
        &self,
        key: (&'query Q0, &'query Q1),
    ) -> TraderTimestampRange<'a>
    where
        String: Borrow<Q0>,
        u64: Borrow<Q1>,
        Q0: Ord + ?Sized,
        Q1: Ord + ?Sized,
    {
        TraderTimestampRange::new(OrderRefs::new(
            &self.map.inner.nodes,
            self.map
                .inner
                .trader_timestamp
                .equal_iter_ids(&key, &self.map.inner.nodes),
        ))
    }

    pub(crate) fn iter(&self) -> TraderTimestampIter<'a> {
        TraderTimestampIter::new(OrderRefs::new(
            &self.map.inner.nodes,
            self.map
                .inner
                .trader_timestamp
                .iter_ids(&self.map.inner.nodes),
        ))
    }

    pub(crate) fn range<'query, Q0, Q1, R>(&self, range: R) -> TraderTimestampRange<'a>
    where
        String: Borrow<Q0>,
        u64: Borrow<Q1>,
        Q0: Ord + ?Sized + 'query,
        Q1: Ord + ?Sized + 'query,
        R: RangeBounds<(&'query Q0, &'query Q1)>,
    {
        TraderTimestampRange::new(OrderRefs::new(
            &self.map.inner.nodes,
            self.map
                .inner
                .trader_timestamp
                .range_iter_ids(range, &self.map.inner.nodes),
        ))
    }
}

pub(crate) struct TraderTimestampViewMut<'a> {
    map: &'a mut OrderMap,
}

impl TraderTimestampViewMut<'_> {
    pub(crate) fn update_each(&mut self, f: impl FnMut(OrderUpdate<'_>)) -> usize {
        let ids = self
            .map
            .inner
            .trader_timestamp
            .iter_ids(&self.map.inner.nodes)
            .collect();
        self.map.inner.update_ids(ids, f)
    }

    pub(crate) fn remove_all<'query, Q0, Q1>(&mut self, key: (&'query Q0, &'query Q1)) -> Vec<Order>
    where
        String: Borrow<Q0>,
        u64: Borrow<Q1>,
        Q0: Ord + ?Sized,
        Q1: Ord + ?Sized,
    {
        let ids = self
            .map
            .inner
            .trader_timestamp
            .equal_ids(&key, &self.map.inner.nodes);
        let orders = ids
            .into_iter()
            .map(|id| self.map.inner.remove_id(id))
            .collect();
        self.map.inner.validate_debug();
        orders
    }

    pub(crate) fn modify_all<'query, Q0, Q1>(
        &mut self,
        key: (&'query Q0, &'query Q1),
        f: impl FnMut(&mut Order),
    ) -> ModifyAllResult
    where
        String: Borrow<Q0>,
        u64: Borrow<Q1>,
        Q0: Ord + ?Sized,
        Q1: Ord + ?Sized,
    {
        let ids = self
            .map
            .inner
            .trader_timestamp
            .equal_ids(&key, &self.map.inner.nodes);
        self.map.inner.modify_ids(ids, f)
    }

    pub(crate) fn update_all<'query, Q0, Q1>(
        &mut self,
        key: (&'query Q0, &'query Q1),
        f: impl FnMut(OrderUpdate<'_>),
    ) -> usize
    where
        String: Borrow<Q0>,
        u64: Borrow<Q1>,
        Q0: Ord + ?Sized,
        Q1: Ord + ?Sized,
    {
        let ids = self
            .map
            .inner
            .trader_timestamp
            .equal_ids(&key, &self.map.inner.nodes);
        self.map.inner.update_ids(ids, f)
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
        self.map.inner.id.len()
    }

    fn iter(&self) -> Self::Iter<'_> {
        IdIter::new(OrderRefs::new(
            &self.map.inner.nodes,
            self.map.inner.id.iter_ids(&self.map.inner.nodes),
        ))
    }
}

impl UniqueView for IdView<'_> {
    fn get(&self, key: &Self::Key) -> Option<&Self::Value> {
        self.map
            .inner
            .id
            .find(key, &self.map.inner.nodes)
            .map(|id| &self.map.inner.nodes[id.0].order)
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
        self.map.inner.id.len()
    }

    fn iter(&self) -> Self::Iter<'_> {
        IdIter::new(OrderRefs::new(
            &self.map.inner.nodes,
            self.map.inner.id.iter_ids(&self.map.inner.nodes),
        ))
    }
}

impl UniqueView for IdViewMut<'_> {
    fn get(&self, key: &Self::Key) -> Option<&Self::Value> {
        self.map
            .inner
            .id
            .find(key, &self.map.inner.nodes)
            .map(|id| &self.map.inner.nodes[id.0].order)
    }
}

impl IndexViewMut for IdViewMut<'_> {
    type Update<'a> = OrderUpdate<'a>;

    fn update_each<F>(&mut self, f: F) -> usize
    where
        F: for<'a> FnMut(Self::Update<'a>),
    {
        IdViewMut::update_each(self, f)
    }
}

impl UniqueViewMut for IdViewMut<'_> {
    type Conflict = Conflict;

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
        self.map.inner.timestamp.len()
    }

    fn iter(&self) -> Self::Iter<'_> {
        TimestampIter::new(OrderRefs::new(
            &self.map.inner.nodes,
            self.map.inner.timestamp.iter_ids(&self.map.inner.nodes),
        ))
    }
}

impl UniqueView for TimestampView<'_> {
    fn get(&self, key: &Self::Key) -> Option<&Self::Value> {
        self.map
            .inner
            .timestamp
            .find(key, &self.map.inner.nodes)
            .map(|id| &self.map.inner.nodes[id.0].order)
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
            &self.map.inner.nodes,
            self.map
                .inner
                .timestamp
                .range_iter_ids(range, &self.map.inner.nodes),
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
        self.map.inner.timestamp.len()
    }

    fn iter(&self) -> Self::Iter<'_> {
        TimestampIter::new(OrderRefs::new(
            &self.map.inner.nodes,
            self.map.inner.timestamp.iter_ids(&self.map.inner.nodes),
        ))
    }
}

impl UniqueView for TimestampViewMut<'_> {
    fn get(&self, key: &Self::Key) -> Option<&Self::Value> {
        self.map
            .inner
            .timestamp
            .find(key, &self.map.inner.nodes)
            .map(|id| &self.map.inner.nodes[id.0].order)
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
            &self.map.inner.nodes,
            self.map
                .inner
                .timestamp
                .range_iter_ids(range, &self.map.inner.nodes),
        ))
    }
}

impl IndexViewMut for TimestampViewMut<'_> {
    type Update<'a> = OrderUpdate<'a>;

    fn update_each<F>(&mut self, f: F) -> usize
    where
        F: for<'a> FnMut(Self::Update<'a>),
    {
        TimestampViewMut::update_each(self, f)
    }
}

impl UniqueViewMut for TimestampViewMut<'_> {
    type Conflict = Conflict;

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
        self.map.inner.trader.len()
    }

    fn iter(&self) -> Self::Iter<'_> {
        TraderIter::new(OrderRefs::new(
            &self.map.inner.nodes,
            self.map.inner.trader.iter_ids(&self.map.inner.nodes),
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
            &self.map.inner.nodes,
            self.map
                .inner
                .trader
                .equal_iter_ids(key, &self.map.inner.nodes),
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
        self.map.inner.trader.len()
    }

    fn iter(&self) -> Self::Iter<'_> {
        TraderIter::new(OrderRefs::new(
            &self.map.inner.nodes,
            self.map.inner.trader.iter_ids(&self.map.inner.nodes),
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
            &self.map.inner.nodes,
            self.map
                .inner
                .trader
                .equal_iter_ids(key, &self.map.inner.nodes),
        ))
    }
}

impl IndexViewMut for TraderViewMut<'_> {
    type Update<'a> = OrderUpdate<'a>;

    fn update_each<F>(&mut self, f: F) -> usize
    where
        F: for<'a> FnMut(Self::Update<'a>),
    {
        TraderViewMut::update_each(self, f)
    }
}

impl NonUniqueViewMut for TraderViewMut<'_> {
    type ModifyAllResult = ModifyAllResult;

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
        self.map.inner.price.len()
    }

    fn iter(&self) -> Self::Iter<'_> {
        PriceIter::new(OrderRefs::new(
            &self.map.inner.nodes,
            self.map.inner.price.iter_ids(&self.map.inner.nodes),
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
            &self.map.inner.nodes,
            self.map
                .inner
                .price
                .range_iter_ids(*key..=*key, &self.map.inner.nodes),
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
            &self.map.inner.nodes,
            self.map
                .inner
                .price
                .range_iter_ids(range, &self.map.inner.nodes),
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
        self.map.inner.price.len()
    }

    fn iter(&self) -> Self::Iter<'_> {
        PriceIter::new(OrderRefs::new(
            &self.map.inner.nodes,
            self.map.inner.price.iter_ids(&self.map.inner.nodes),
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
            &self.map.inner.nodes,
            self.map
                .inner
                .price
                .range_iter_ids(*key..=*key, &self.map.inner.nodes),
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
            &self.map.inner.nodes,
            self.map
                .inner
                .price
                .range_iter_ids(range, &self.map.inner.nodes),
        ))
    }
}

impl IndexViewMut for PriceViewMut<'_> {
    type Update<'a> = OrderUpdate<'a>;

    fn update_each<F>(&mut self, f: F) -> usize
    where
        F: for<'a> FnMut(Self::Update<'a>),
    {
        PriceViewMut::update_each(self, f)
    }
}

impl NonUniqueViewMut for PriceViewMut<'_> {
    type ModifyAllResult = ModifyAllResult;

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

impl IndexView for TraderTimestampView<'_> {
    type Value = Order;
    type Key = (String, u64);
    type Iter<'a>
        = TraderTimestampIter<'a>
    where
        Self: 'a;

    fn len(&self) -> usize {
        self.map.inner.trader_timestamp.len()
    }

    fn iter(&self) -> Self::Iter<'_> {
        TraderTimestampIter::new(OrderRefs::new(
            &self.map.inner.nodes,
            self.map
                .inner
                .trader_timestamp
                .iter_ids(&self.map.inner.nodes),
        ))
    }
}

impl IndexView for TraderTimestampViewMut<'_> {
    type Value = Order;
    type Key = (String, u64);
    type Iter<'a>
        = TraderTimestampIter<'a>
    where
        Self: 'a;

    fn len(&self) -> usize {
        self.map.inner.trader_timestamp.len()
    }

    fn iter(&self) -> Self::Iter<'_> {
        TraderTimestampIter::new(OrderRefs::new(
            &self.map.inner.nodes,
            self.map
                .inner
                .trader_timestamp
                .iter_ids(&self.map.inner.nodes),
        ))
    }
}

impl IndexViewMut for TraderTimestampViewMut<'_> {
    type Update<'a> = OrderUpdate<'a>;

    fn update_each<F>(&mut self, f: F) -> usize
    where
        F: for<'a> FnMut(Self::Update<'a>),
    {
        TraderTimestampViewMut::update_each(self, f)
    }
}
