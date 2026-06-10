use slab::Slab;
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::hash::{BuildHasher, Hash};
use std::marker::PhantomData;
use std::ops::{Bound, RangeBounds};

#[cfg(feature = "rustc-hash")]
pub type DefaultHashBuilder = rustc_hash::FxBuildHasher;
#[cfg(not(feature = "rustc-hash"))]
pub type DefaultHashBuilder = std::hash::RandomState;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, Default)]
pub struct HashLink {
    prev: Option<NodeId>,
    next: Option<NodeId>,
    hash: u64,
    linked: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Color {
    Red,
    #[default]
    Black,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct OrderedLink {
    parent: Option<NodeId>,
    left: Option<NodeId>,
    right: Option<NodeId>,
    color: Color,
    linked: bool,
}

pub trait NodeValue {
    type Value;

    fn value(&self) -> &Self::Value;
}

#[doc(hidden)]
pub struct DebugValues<'a, N> {
    nodes: &'a Slab<N>,
}

impl<'a, N> DebugValues<'a, N> {
    pub fn new(nodes: &'a Slab<N>) -> Self {
        Self { nodes }
    }
}

impl<N> std::fmt::Debug for DebugValues<'_, N>
where
    N: NodeValue,
    N::Value: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list()
            .entries(self.nodes.iter().map(|(_, node)| node.value()))
            .finish()
    }
}

pub trait Equivalent<Q: ?Sized> {
    fn equivalent(&self, query: &Q) -> bool;
}

pub trait Compare<Q: ?Sized> {
    fn compare(&self, query: &Q) -> Ordering;
}

pub struct QueryRange<Q> {
    start: Bound<Q>,
    end: Bound<Q>,
}

impl<Q> QueryRange<Q> {
    pub fn new(start: Bound<Q>, end: Bound<Q>) -> Self {
        Self { start, end }
    }
}

impl<Q> RangeBounds<Q> for QueryRange<Q> {
    fn start_bound(&self) -> Bound<&Q> {
        self.start.as_ref()
    }

    fn end_bound(&self) -> Bound<&Q> {
        self.end.as_ref()
    }
}

impl<T, Q> Equivalent<Q> for &T
where
    T: Borrow<Q>,
    Q: Eq + ?Sized,
{
    fn equivalent(&self, query: &Q) -> bool {
        (*self).borrow() == query
    }
}

impl<T, Q> Compare<Q> for &T
where
    T: Borrow<Q>,
    Q: Ord + ?Sized,
{
    fn compare(&self, query: &Q) -> Ordering {
        (*self).borrow().cmp(query)
    }
}

macro_rules! impl_tuple_query {
    ($($t:ident $q:ident $index:tt),+) => {
        impl<$($t, $q),+> Equivalent<($(&$q),+)> for ($(&$t),+)
        where
            $($t: Borrow<$q>, $q: Eq + ?Sized),+
        {
            fn equivalent(&self, query: &($(&$q),+)) -> bool {
                $(self.$index.borrow() == query.$index)&&+
            }
        }

        impl<$($t, $q),+> Compare<($(&$q),+)> for ($(&$t),+)
        where
            $($t: Borrow<$q>, $q: Ord + ?Sized),+
        {
            fn compare(&self, query: &($(&$q),+)) -> Ordering {
                $(
                    match self.$index.borrow().cmp(query.$index) {
                        Ordering::Equal => {}
                        ordering => return ordering,
                    }
                )+
                Ordering::Equal
            }
        }
    };
}

impl_tuple_query!(T0 Q0 0, T1 Q1 1);
impl_tuple_query!(T0 Q0 0, T1 Q1 1, T2 Q2 2);
impl_tuple_query!(T0 Q0 0, T1 Q1 1, T2 Q2 2, T3 Q3 3);
impl_tuple_query!(T0 Q0 0, T1 Q1 1, T2 Q2 2, T3 Q3 3, T4 Q4 4);
impl_tuple_query!(T0 Q0 0, T1 Q1 1, T2 Q2 2, T3 Q3 3, T4 Q4 4, T5 Q5 5);
impl_tuple_query!(T0 Q0 0, T1 Q1 1, T2 Q2 2, T3 Q3 3, T4 Q4 4, T5 Q5 5, T6 Q6 6);
impl_tuple_query!(T0 Q0 0, T1 Q1 1, T2 Q2 2, T3 Q3 3, T4 Q4 4, T5 Q5 5, T6 Q6 6, T7 Q7 7);
impl_tuple_query!(T0 Q0 0, T1 Q1 1, T2 Q2 2, T3 Q3 3, T4 Q4 4, T5 Q5 5, T6 Q6 6, T7 Q7 7, T8 Q8 8);
impl_tuple_query!(T0 Q0 0, T1 Q1 1, T2 Q2 2, T3 Q3 3, T4 Q4 4, T5 Q5 5, T6 Q6 6, T7 Q7 7, T8 Q8 8, T9 Q9 9);
impl_tuple_query!(T0 Q0 0, T1 Q1 1, T2 Q2 2, T3 Q3 3, T4 Q4 4, T5 Q5 5, T6 Q6 6, T7 Q7 7, T8 Q8 8, T9 Q9 9, T10 Q10 10);
impl_tuple_query!(T0 Q0 0, T1 Q1 1, T2 Q2 2, T3 Q3 3, T4 Q4 4, T5 Q5 5, T6 Q6 6, T7 Q7 7, T8 Q8 8, T9 Q9 9, T10 Q10 10, T11 Q11 11);

pub trait IndexSpec<N: NodeValue> {
    type Key<'a>;
    type Link;

    const NAME: &'static str;

    fn key(value: &N::Value) -> Self::Key<'_>;
    fn link(node: &N) -> &Self::Link;
    fn link_mut(node: &mut N) -> &mut Self::Link;
}

pub struct HashedUnique;
pub struct HashedNonUnique;
pub struct OrderedUnique;
pub struct OrderedNonUnique;

pub trait IndexCategory {
    type Link: Clone + Default;
}

impl IndexCategory for HashedUnique {
    type Link = HashLink;
}

impl IndexCategory for HashedNonUnique {
    type Link = HashLink;
}

impl IndexCategory for OrderedUnique {
    type Link = OrderedLink;
}

impl IndexCategory for OrderedNonUnique {
    type Link = OrderedLink;
}

pub trait UniqueCategory {}
pub trait NonUniqueCategory {}
pub trait OrderedCategory {}
pub trait CompatibilityKind {
    type Collection<T>;

    fn from_vec<T>(values: Vec<T>) -> Self::Collection<T>;
}

impl UniqueCategory for HashedUnique {}
impl UniqueCategory for OrderedUnique {}
impl NonUniqueCategory for HashedNonUnique {}
impl NonUniqueCategory for OrderedNonUnique {}
impl OrderedCategory for OrderedUnique {}
impl OrderedCategory for OrderedNonUnique {}

impl CompatibilityKind for HashedUnique {
    type Collection<T> = Option<T>;

    fn from_vec<T>(values: Vec<T>) -> Self::Collection<T> {
        values.into_iter().next()
    }
}

impl CompatibilityKind for OrderedUnique {
    type Collection<T> = Option<T>;

    fn from_vec<T>(values: Vec<T>) -> Self::Collection<T> {
        values.into_iter().next()
    }
}

impl CompatibilityKind for HashedNonUnique {
    type Collection<T> = Vec<T>;

    fn from_vec<T>(values: Vec<T>) -> Self::Collection<T> {
        values
    }
}

impl CompatibilityKind for OrderedNonUnique {
    type Collection<T> = Vec<T>;

    fn from_vec<T>(values: Vec<T>) -> Self::Collection<T> {
        values
    }
}

pub trait IndexKind<N, S>: IndexCategory
where
    N: NodeValue,
    S: IndexSpec<N, Link = Self::Link>,
{
    type Index: Clone + Default + 'static;
    type Ids<'a>: Iterator<Item = NodeId>
    where
        N: 'a,
        S: 'a,
        Self::Index: 'a;
    type EqualIds<'a>: Iterator<Item = NodeId>
    where
        N: 'a,
        S: 'a,
        Self::Index: 'a;

    fn len(index: &Self::Index) -> usize;
    fn clear(index: &mut Self::Index);
    fn reserve_for_insert(index: &mut Self::Index, nodes: &mut Slab<N>);
    fn iter_ids<'a>(index: &'a Self::Index, nodes: &'a Slab<N>) -> Self::Ids<'a>;
    fn insert(index: &mut Self::Index, id: NodeId, nodes: &mut Slab<N>) -> Result<(), NodeId>;
    fn remove(index: &mut Self::Index, id: NodeId, nodes: &mut Slab<N>);
    fn reconcile(index: &mut Self::Index, id: NodeId, nodes: &mut Slab<N>) -> Result<(), NodeId>;
    fn validate(index: &Self::Index, nodes: &Slab<N>) -> Result<(), String>;
}

pub trait QueryIndexKind<N, S, Q: ?Sized>: IndexKind<N, S>
where
    N: NodeValue,
    S: IndexSpec<N, Link = Self::Link>,
{
    fn find(index: &Self::Index, key: &Q, nodes: &Slab<N>) -> Option<NodeId>;
    fn equal_ids(index: &Self::Index, key: &Q, nodes: &Slab<N>) -> Vec<NodeId>;
    fn equal_iter_ids<'a>(
        index: &'a Self::Index,
        key: &Q,
        nodes: &'a Slab<N>,
    ) -> Self::EqualIds<'a>;
}

pub trait OrderedIndexKind<N, S>: IndexKind<N, S>
where
    N: NodeValue,
    S: IndexSpec<N, Link = Self::Link>,
{
    type RangeIds<'a>: DoubleEndedIterator<Item = NodeId>
    where
        N: 'a,
        S: 'a,
        Self::Index: 'a;

    fn range_iter_ids<'a, Q, R>(
        index: &'a Self::Index,
        range: R,
        nodes: &'a Slab<N>,
    ) -> Self::RangeIds<'a>
    where
        R: RangeBounds<Q>,
        Q: Ord + ?Sized,
        for<'key> S::Key<'key>: Compare<Q>;
}

pub struct HashedIndex<S, const UNIQUE: bool, H = DefaultHashBuilder> {
    buckets: Vec<Option<NodeId>>,
    len: usize,
    hash_builder: H,
    marker: PhantomData<S>,
}

impl<S, const UNIQUE: bool, H: Clone> Clone for HashedIndex<S, UNIQUE, H> {
    fn clone(&self) -> Self {
        Self {
            buckets: self.buckets.clone(),
            len: self.len,
            hash_builder: self.hash_builder.clone(),
            marker: PhantomData,
        }
    }
}

impl<S, const UNIQUE: bool, H: Default> Default for HashedIndex<S, UNIQUE, H> {
    fn default() -> Self {
        Self {
            buckets: vec![None; 8],
            len: 0,
            hash_builder: H::default(),
            marker: PhantomData,
        }
    }
}

pub struct HashIds<'a, N, S, const UNIQUE: bool, H = DefaultHashBuilder> {
    index: &'a HashedIndex<S, UNIQUE, H>,
    nodes: &'a Slab<N>,
    bucket: usize,
    current: Option<NodeId>,
    remaining: usize,
}

impl<N, S, const UNIQUE: bool, H> Iterator for HashIds<'_, N, S, UNIQUE, H>
where
    N: NodeValue,
    S: IndexSpec<N, Link = HashLink>,
{
    type Item = NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(id) = self.current {
                self.current = S::link(&self.nodes[id.0]).next;
                self.remaining -= 1;
                return Some(id);
            }
            self.current = *self.index.buckets.get(self.bucket)?;
            self.bucket += 1;
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<N, S, const UNIQUE: bool, H> ExactSizeIterator for HashIds<'_, N, S, UNIQUE, H>
where
    N: NodeValue,
    S: IndexSpec<N, Link = HashLink>,
{
}

impl<N, S, const UNIQUE: bool, H> std::iter::FusedIterator for HashIds<'_, N, S, UNIQUE, H>
where
    N: NodeValue,
    S: IndexSpec<N, Link = HashLink>,
{
}

pub struct HashEqualIds<'a, N, S, const UNIQUE: bool, H = DefaultHashBuilder> {
    nodes: &'a Slab<N>,
    current: Option<NodeId>,
    remaining: usize,
    marker: PhantomData<(&'a HashedIndex<S, UNIQUE, H>, S)>,
}

impl<N, S, const UNIQUE: bool, H> Iterator for HashEqualIds<'_, N, S, UNIQUE, H>
where
    N: NodeValue,
    S: IndexSpec<N, Link = HashLink>,
{
    type Item = NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        let id = self.current?;
        self.current = S::link(&self.nodes[id.0]).next;
        self.remaining -= 1;
        Some(id)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<N, S, const UNIQUE: bool, H> ExactSizeIterator for HashEqualIds<'_, N, S, UNIQUE, H>
where
    N: NodeValue,
    S: IndexSpec<N, Link = HashLink>,
{
}

impl<N, S, const UNIQUE: bool, H> std::iter::FusedIterator for HashEqualIds<'_, N, S, UNIQUE, H>
where
    N: NodeValue,
    S: IndexSpec<N, Link = HashLink>,
{
}

impl<S, const UNIQUE: bool, H> HashedIndex<S, UNIQUE, H>
where
    H: BuildHasher,
{
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn clear(&mut self) {
        self.buckets.fill(None);
        self.len = 0;
    }

    fn hash<Q: Hash + ?Sized>(&self, key: &Q) -> u64 {
        self.hash_builder.hash_one(key)
    }

    fn bucket(&self, hash: u64) -> usize {
        hash as usize % self.buckets.len()
    }

    pub fn reserve_for_insert<N>(&mut self, nodes: &mut Slab<N>)
    where
        N: NodeValue,
        S: IndexSpec<N, Link = HashLink>,
        for<'a> S::Key<'a>: Eq + Hash,
    {
        if (self.len + 1) * 4 > self.buckets.len() * 3 {
            self.rehash(nodes, self.buckets.len() * 2);
        }
    }

    fn rehash<N>(&mut self, nodes: &mut Slab<N>, bucket_count: usize)
    where
        N: NodeValue,
        S: IndexSpec<N, Link = HashLink>,
        for<'a> S::Key<'a>: Eq + Hash,
    {
        let ids: Vec<_> = nodes
            .iter()
            .filter_map(|(id, node)| S::link(node).linked.then_some(NodeId(id)))
            .collect();

        self.buckets = vec![None; bucket_count.max(8)];
        self.len = 0;
        for id in ids {
            *S::link_mut(&mut nodes[id.0]) = HashLink::default();
            let inserted = self.insert(id, nodes);
            debug_assert!(inserted.is_ok());
        }
    }

    pub fn find<N, Q>(&self, key: &Q, nodes: &Slab<N>) -> Option<NodeId>
    where
        N: NodeValue,
        S: IndexSpec<N, Link = HashLink>,
        for<'a> S::Key<'a>: Equivalent<Q>,
        Q: Eq + Hash + ?Sized,
    {
        let hash = self.hash(key);
        let mut current = self.buckets[self.bucket(hash)];
        while let Some(id) = current {
            let node = &nodes[id.0];
            let link = S::link(node);
            if link.hash == hash && S::key(node.value()).equivalent(key) {
                return Some(id);
            }
            current = link.next;
        }
        None
    }

    pub fn equal_ids<N, Q>(&self, key: &Q, nodes: &Slab<N>) -> Vec<NodeId>
    where
        N: NodeValue,
        S: IndexSpec<N, Link = HashLink>,
        for<'a> S::Key<'a>: Equivalent<Q>,
        Q: Eq + Hash + ?Sized,
    {
        let hash = self.hash(key);
        let mut current = self.buckets[self.bucket(hash)];
        let mut ids = Vec::new();
        while let Some(id) = current {
            let node = &nodes[id.0];
            let link = S::link(node);
            if link.hash == hash && S::key(node.value()).equivalent(key) {
                ids.push(id);
            } else if !ids.is_empty() {
                break;
            }
            current = link.next;
        }
        ids
    }

    pub fn iter_ids<'a, N>(&'a self, nodes: &'a Slab<N>) -> HashIds<'a, N, S, UNIQUE, H>
    where
        N: NodeValue,
        S: IndexSpec<N, Link = HashLink>,
    {
        HashIds {
            index: self,
            nodes,
            bucket: 0,
            current: None,
            remaining: self.len,
        }
    }

    pub fn equal_iter_ids<'a, N, Q>(
        &'a self,
        key: &Q,
        nodes: &'a Slab<N>,
    ) -> HashEqualIds<'a, N, S, UNIQUE, H>
    where
        N: NodeValue,
        S: IndexSpec<N, Link = HashLink>,
        for<'k> S::Key<'k>: Equivalent<Q>,
        Q: Eq + Hash + ?Sized,
    {
        let first = self.find(key, nodes);
        let mut remaining = 0;
        let mut current = first;
        while let Some(id) = current {
            let node = &nodes[id.0];
            let link = S::link(node);
            if !S::key(node.value()).equivalent(key) {
                break;
            }
            remaining += 1;
            current = link.next;
        }
        HashEqualIds {
            nodes,
            current: first,
            remaining,
            marker: PhantomData,
        }
    }

    pub fn insert<N>(&mut self, id: NodeId, nodes: &mut Slab<N>) -> Result<(), NodeId>
    where
        N: NodeValue,
        S: IndexSpec<N, Link = HashLink>,
        for<'a> S::Key<'a>: Eq + Hash,
    {
        let hash = self.hash(&S::key(nodes[id.0].value()));
        let bucket = self.bucket(hash);
        let mut current = self.buckets[bucket];
        let mut equal_last = None;

        while let Some(other) = current {
            let node = &nodes[other.0];
            let link = S::link(node);
            if link.hash == hash && S::key(node.value()) == S::key(nodes[id.0].value()) {
                if UNIQUE {
                    return Err(other);
                }
                equal_last = Some(other);
            } else if equal_last.is_some() {
                break;
            }
            current = link.next;
        }

        let (prev, next) = if let Some(last) = equal_last {
            (Some(last), S::link(&nodes[last.0]).next)
        } else {
            (None, self.buckets[bucket])
        };

        *S::link_mut(&mut nodes[id.0]) = HashLink {
            prev,
            next,
            hash,
            linked: true,
        };
        if let Some(prev) = prev {
            S::link_mut(&mut nodes[prev.0]).next = Some(id);
        } else {
            self.buckets[bucket] = Some(id);
        }
        if let Some(next) = next {
            S::link_mut(&mut nodes[next.0]).prev = Some(id);
        }
        self.len += 1;
        Ok(())
    }

    pub fn remove<N>(&mut self, id: NodeId, nodes: &mut Slab<N>)
    where
        N: NodeValue,
        S: IndexSpec<N, Link = HashLink>,
    {
        let link = *S::link(&nodes[id.0]);
        if !link.linked {
            return;
        }
        if let Some(prev) = link.prev {
            S::link_mut(&mut nodes[prev.0]).next = link.next;
        } else {
            let bucket = self.bucket(link.hash);
            self.buckets[bucket] = link.next;
        }
        if let Some(next) = link.next {
            S::link_mut(&mut nodes[next.0]).prev = link.prev;
        }
        *S::link_mut(&mut nodes[id.0]) = HashLink::default();
        self.len -= 1;
    }

    pub fn reconcile<N>(&mut self, id: NodeId, nodes: &mut Slab<N>) -> Result<(), NodeId>
    where
        N: NodeValue,
        S: IndexSpec<N, Link = HashLink>,
        for<'a> S::Key<'a>: Eq + Hash,
    {
        if self.in_place(id, nodes) {
            return Ok(());
        }
        self.remove(id, nodes);
        self.insert(id, nodes)
    }

    fn in_place<N>(&self, id: NodeId, nodes: &Slab<N>) -> bool
    where
        N: NodeValue,
        S: IndexSpec<N, Link = HashLink>,
        for<'a> S::Key<'a>: Eq + Hash,
    {
        let key = S::key(nodes[id.0].value());
        let link = S::link(&nodes[id.0]);
        let hash = self.hash(&key);
        if hash != link.hash {
            return false;
        }

        let mut current = self.buckets[self.bucket(hash)];
        let mut seen_equal = false;
        let mut left_equal_group = false;
        let mut saw_id = false;
        while let Some(other) = current {
            let other_node = &nodes[other.0];
            let other_link = S::link(other_node);
            let equal = other_link.hash == hash && S::key(other_node.value()) == key;
            if equal {
                if left_equal_group {
                    return false;
                }
                if UNIQUE && other != id {
                    return false;
                }
                seen_equal = true;
                saw_id |= other == id;
            } else if seen_equal {
                left_equal_group = true;
            }
            current = other_link.next;
        }
        saw_id
    }

    pub fn validate<N>(&self, nodes: &Slab<N>) -> Result<(), String>
    where
        N: NodeValue,
        S: IndexSpec<N, Link = HashLink>,
        for<'a> S::Key<'a>: Eq + Hash,
    {
        let mut seen = vec![false; nodes.capacity().max(1)];
        let mut count = 0;
        for (bucket, head) in self.buckets.iter().enumerate() {
            let mut current = *head;
            let mut prev = None;
            let mut groups: Vec<NodeId> = Vec::new();
            while let Some(id) = current {
                if id.0 >= seen.len() || seen[id.0] {
                    return Err(format!("{} contains a cycle or duplicate node", S::NAME));
                }
                seen[id.0] = true;
                let node = nodes
                    .get(id.0)
                    .ok_or_else(|| format!("{} points outside the arena", S::NAME))?;
                let link = S::link(node);
                if !link.linked || link.prev != prev {
                    return Err(format!("{} has inconsistent hash links", S::NAME));
                }
                if self.bucket(link.hash) != bucket || self.hash(&S::key(node.value())) != link.hash
                {
                    return Err(format!("{} has a stale cached hash", S::NAME));
                }
                let key = S::key(node.value());
                if groups
                    .last()
                    .is_none_or(|last| S::key(nodes[last.0].value()) != key)
                {
                    if groups
                        .iter()
                        .any(|group| S::key(nodes[group.0].value()) == key)
                    {
                        return Err(format!("{} has a split equivalent-key group", S::NAME));
                    }
                    groups.push(id);
                } else if UNIQUE {
                    return Err(format!("{} violates uniqueness", S::NAME));
                }
                count += 1;
                prev = current;
                current = link.next;
            }
        }
        if count != self.len {
            return Err(format!("{} count does not match its links", S::NAME));
        }
        for (id, node) in nodes.iter() {
            if S::link(node).linked != seen.get(id).copied().unwrap_or(false) {
                return Err(format!("{} membership does not match the arena", S::NAME));
            }
        }
        Ok(())
    }
}

pub struct OrderedIndex<S, const UNIQUE: bool> {
    root: Option<NodeId>,
    first: Option<NodeId>,
    last: Option<NodeId>,
    len: usize,
    marker: PhantomData<S>,
}

impl<S, const UNIQUE: bool> Clone for OrderedIndex<S, UNIQUE> {
    fn clone(&self) -> Self {
        Self {
            root: self.root,
            first: self.first,
            last: self.last,
            len: self.len,
            marker: PhantomData,
        }
    }
}

impl<S, const UNIQUE: bool> Default for OrderedIndex<S, UNIQUE> {
    fn default() -> Self {
        Self {
            root: None,
            first: None,
            last: None,
            len: 0,
            marker: PhantomData,
        }
    }
}

pub struct OrderedIds<'a, N, S, const UNIQUE: bool> {
    index: &'a OrderedIndex<S, UNIQUE>,
    nodes: &'a Slab<N>,
    front: Option<NodeId>,
    back: Option<NodeId>,
    remaining: usize,
}

impl<N, S, const UNIQUE: bool> Iterator for OrderedIds<'_, N, S, UNIQUE>
where
    N: NodeValue,
    S: IndexSpec<N, Link = OrderedLink>,
{
    type Item = NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.front?;
        self.remaining -= 1;
        self.front = if self.remaining == 0 {
            None
        } else {
            self.index.successor(id, self.nodes)
        };
        Some(id)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<N, S, const UNIQUE: bool> DoubleEndedIterator for OrderedIds<'_, N, S, UNIQUE>
where
    N: NodeValue,
    S: IndexSpec<N, Link = OrderedLink>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        let id = self.back?;
        self.remaining -= 1;
        self.back = if self.remaining == 0 {
            None
        } else {
            self.index.predecessor(id, self.nodes)
        };
        Some(id)
    }
}

impl<N, S, const UNIQUE: bool> ExactSizeIterator for OrderedIds<'_, N, S, UNIQUE>
where
    N: NodeValue,
    S: IndexSpec<N, Link = OrderedLink>,
{
}

impl<N, S, const UNIQUE: bool> std::iter::FusedIterator for OrderedIds<'_, N, S, UNIQUE>
where
    N: NodeValue,
    S: IndexSpec<N, Link = OrderedLink>,
{
}

pub struct OrderedRangeIds<'a, N, S, const UNIQUE: bool> {
    index: &'a OrderedIndex<S, UNIQUE>,
    nodes: &'a Slab<N>,
    front: Option<NodeId>,
    back: Option<NodeId>,
    done: bool,
}

impl<N, S, const UNIQUE: bool> Iterator for OrderedRangeIds<'_, N, S, UNIQUE>
where
    N: NodeValue,
    S: IndexSpec<N, Link = OrderedLink>,
{
    type Item = NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let id = self.front?;
        if Some(id) == self.back {
            self.done = true;
        } else {
            self.front = self.index.successor(id, self.nodes);
        }
        Some(id)
    }
}

impl<N, S, const UNIQUE: bool> DoubleEndedIterator for OrderedRangeIds<'_, N, S, UNIQUE>
where
    N: NodeValue,
    S: IndexSpec<N, Link = OrderedLink>,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let id = self.back?;
        if Some(id) == self.front {
            self.done = true;
        } else {
            self.back = self.index.predecessor(id, self.nodes);
        }
        Some(id)
    }
}

impl<N, S, const UNIQUE: bool> std::iter::FusedIterator for OrderedRangeIds<'_, N, S, UNIQUE>
where
    N: NodeValue,
    S: IndexSpec<N, Link = OrderedLink>,
{
}

impl<S, const UNIQUE: bool> OrderedIndex<S, UNIQUE> {
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn clear(&mut self) {
        self.root = None;
        self.first = None;
        self.last = None;
        self.len = 0;
    }

    fn color<N>(&self, id: Option<NodeId>, nodes: &Slab<N>) -> Color
    where
        N: NodeValue,
        S: IndexSpec<N, Link = OrderedLink>,
    {
        id.map_or(Color::Black, |id| S::link(&nodes[id.0]).color)
    }

    fn set_color<N>(&self, id: Option<NodeId>, color: Color, nodes: &mut Slab<N>)
    where
        N: NodeValue,
        S: IndexSpec<N, Link = OrderedLink>,
    {
        if let Some(id) = id {
            S::link_mut(&mut nodes[id.0]).color = color;
        }
    }

    fn parent<N>(&self, id: Option<NodeId>, nodes: &Slab<N>) -> Option<NodeId>
    where
        N: NodeValue,
        S: IndexSpec<N, Link = OrderedLink>,
    {
        id.and_then(|id| S::link(&nodes[id.0]).parent)
    }

    fn left<N>(&self, id: Option<NodeId>, nodes: &Slab<N>) -> Option<NodeId>
    where
        N: NodeValue,
        S: IndexSpec<N, Link = OrderedLink>,
    {
        id.and_then(|id| S::link(&nodes[id.0]).left)
    }

    fn right<N>(&self, id: Option<NodeId>, nodes: &Slab<N>) -> Option<NodeId>
    where
        N: NodeValue,
        S: IndexSpec<N, Link = OrderedLink>,
    {
        id.and_then(|id| S::link(&nodes[id.0]).right)
    }

    pub fn find<N, Q>(&self, key: &Q, nodes: &Slab<N>) -> Option<NodeId>
    where
        N: NodeValue,
        S: IndexSpec<N, Link = OrderedLink>,
        for<'a> S::Key<'a>: Compare<Q>,
        Q: Ord + ?Sized,
    {
        let mut current = self.root;
        let mut found = None;
        while let Some(id) = current {
            match S::key(nodes[id.0].value()).compare(key) {
                Ordering::Less => current = S::link(&nodes[id.0]).right,
                Ordering::Greater => current = S::link(&nodes[id.0]).left,
                Ordering::Equal => {
                    found = Some(id);
                    current = S::link(&nodes[id.0]).left;
                }
            }
        }
        found
    }

    pub fn equal_ids<N, Q>(&self, key: &Q, nodes: &Slab<N>) -> Vec<NodeId>
    where
        N: NodeValue,
        S: IndexSpec<N, Link = OrderedLink>,
        for<'a> S::Key<'a>: Compare<Q>,
        Q: Ord + ?Sized,
    {
        let mut ids = Vec::new();
        let mut current = self.lower_bound(key, nodes);
        while let Some(id) = current {
            if S::key(nodes[id.0].value()).compare(key) != Ordering::Equal {
                break;
            }
            ids.push(id);
            current = self.successor(id, nodes);
        }
        ids
    }

    pub fn equal_iter_ids<'a, N, Q>(
        &'a self,
        key: &Q,
        nodes: &'a Slab<N>,
    ) -> OrderedRangeIds<'a, N, S, UNIQUE>
    where
        N: NodeValue,
        S: IndexSpec<N, Link = OrderedLink>,
        for<'k> S::Key<'k>: Compare<Q>,
        Q: Ord + ?Sized,
    {
        let front = self.lower_bound(key, nodes);
        let back = match self.upper_bound(key, nodes) {
            Some(id) => self.predecessor(id, nodes),
            None => self.last,
        };
        let done = match (front, back) {
            (Some(front), Some(back)) => {
                S::key(nodes[front.0].value()).compare(key) != Ordering::Equal
                    || S::key(nodes[back.0].value()).compare(key) != Ordering::Equal
            }
            _ => true,
        };
        OrderedRangeIds {
            index: self,
            nodes,
            front,
            back,
            done,
        }
    }

    pub fn ids<N>(&self, nodes: &Slab<N>) -> Vec<NodeId>
    where
        N: NodeValue,
        S: IndexSpec<N, Link = OrderedLink>,
    {
        let mut ids = Vec::with_capacity(self.len);
        let mut current = self.first;
        while let Some(id) = current {
            ids.push(id);
            current = self.successor(id, nodes);
        }
        ids
    }

    pub fn iter_ids<'a, N>(&'a self, nodes: &'a Slab<N>) -> OrderedIds<'a, N, S, UNIQUE>
    where
        N: NodeValue,
        S: IndexSpec<N, Link = OrderedLink>,
    {
        OrderedIds {
            index: self,
            nodes,
            front: self.first,
            back: self.last,
            remaining: self.len,
        }
    }

    pub fn range_iter_ids<'a, N, Q, R>(
        &'a self,
        range: R,
        nodes: &'a Slab<N>,
    ) -> OrderedRangeIds<'a, N, S, UNIQUE>
    where
        N: NodeValue,
        S: IndexSpec<N, Link = OrderedLink>,
        for<'k> S::Key<'k>: Compare<Q>,
        Q: Ord + ?Sized,
        R: RangeBounds<Q>,
        for<'k> S::Key<'k>: Ord,
    {
        let front = match range.start_bound() {
            Bound::Included(key) => self.lower_bound(key, nodes),
            Bound::Excluded(key) => self.upper_bound(key, nodes),
            Bound::Unbounded => self.first,
        };
        let back = match range.end_bound() {
            Bound::Included(key) => match self.upper_bound(key, nodes) {
                Some(id) => self.predecessor(id, nodes),
                None => self.last,
            },
            Bound::Excluded(key) => match self.lower_bound(key, nodes) {
                Some(id) => self.predecessor(id, nodes),
                None => self.last,
            },
            Bound::Unbounded => self.last,
        };
        let done = match (front, back) {
            (Some(front), Some(back)) => {
                S::key(nodes[front.0].value()) > S::key(nodes[back.0].value())
            }
            _ => true,
        };
        OrderedRangeIds {
            index: self,
            nodes,
            front,
            back,
            done,
        }
    }

    fn lower_bound<N, Q>(&self, key: &Q, nodes: &Slab<N>) -> Option<NodeId>
    where
        N: NodeValue,
        S: IndexSpec<N, Link = OrderedLink>,
        for<'a> S::Key<'a>: Compare<Q>,
        Q: Ord + ?Sized,
    {
        let mut current = self.root;
        let mut result = None;
        while let Some(id) = current {
            if S::key(nodes[id.0].value()).compare(key) == Ordering::Less {
                current = S::link(&nodes[id.0]).right;
            } else {
                result = Some(id);
                current = S::link(&nodes[id.0]).left;
            }
        }
        result
    }

    fn upper_bound<N, Q>(&self, key: &Q, nodes: &Slab<N>) -> Option<NodeId>
    where
        N: NodeValue,
        S: IndexSpec<N, Link = OrderedLink>,
        for<'a> S::Key<'a>: Compare<Q>,
        Q: Ord + ?Sized,
    {
        let mut current = self.root;
        let mut result = None;
        while let Some(id) = current {
            if S::key(nodes[id.0].value()).compare(key) != Ordering::Greater {
                current = S::link(&nodes[id.0]).right;
            } else {
                result = Some(id);
                current = S::link(&nodes[id.0]).left;
            }
        }
        result
    }

    fn minimum<N>(&self, mut id: NodeId, nodes: &Slab<N>) -> NodeId
    where
        N: NodeValue,
        S: IndexSpec<N, Link = OrderedLink>,
    {
        while let Some(left) = S::link(&nodes[id.0]).left {
            id = left;
        }
        id
    }

    fn maximum<N>(&self, mut id: NodeId, nodes: &Slab<N>) -> NodeId
    where
        N: NodeValue,
        S: IndexSpec<N, Link = OrderedLink>,
    {
        while let Some(right) = S::link(&nodes[id.0]).right {
            id = right;
        }
        id
    }

    fn successor<N>(&self, id: NodeId, nodes: &Slab<N>) -> Option<NodeId>
    where
        N: NodeValue,
        S: IndexSpec<N, Link = OrderedLink>,
    {
        if let Some(right) = S::link(&nodes[id.0]).right {
            return Some(self.minimum(right, nodes));
        }
        let mut child = id;
        let mut parent = S::link(&nodes[child.0]).parent;
        while let Some(parent_id) = parent {
            if S::link(&nodes[parent_id.0]).left == Some(child) {
                return Some(parent_id);
            }
            child = parent_id;
            parent = S::link(&nodes[child.0]).parent;
        }
        None
    }

    fn predecessor<N>(&self, id: NodeId, nodes: &Slab<N>) -> Option<NodeId>
    where
        N: NodeValue,
        S: IndexSpec<N, Link = OrderedLink>,
    {
        if let Some(left) = S::link(&nodes[id.0]).left {
            return Some(self.maximum(left, nodes));
        }
        let mut child = id;
        let mut parent = S::link(&nodes[child.0]).parent;
        while let Some(parent_id) = parent {
            if S::link(&nodes[parent_id.0]).right == Some(child) {
                return Some(parent_id);
            }
            child = parent_id;
            parent = S::link(&nodes[child.0]).parent;
        }
        None
    }

    pub fn insert<N>(&mut self, id: NodeId, nodes: &mut Slab<N>) -> Result<(), NodeId>
    where
        N: NodeValue,
        S: IndexSpec<N, Link = OrderedLink>,
        for<'a> S::Key<'a>: Ord,
    {
        let mut parent = None;
        let mut current = self.root;
        let mut place_left = false;
        while let Some(other) = current {
            parent = Some(other);
            match S::key(nodes[id.0].value()).cmp(&S::key(nodes[other.0].value())) {
                Ordering::Less => {
                    place_left = true;
                    current = S::link(&nodes[other.0]).left;
                }
                Ordering::Equal if UNIQUE => return Err(other),
                Ordering::Equal | Ordering::Greater => {
                    place_left = false;
                    current = S::link(&nodes[other.0]).right;
                }
            }
        }

        *S::link_mut(&mut nodes[id.0]) = OrderedLink {
            parent,
            color: Color::Red,
            linked: true,
            ..OrderedLink::default()
        };
        if let Some(parent) = parent {
            if place_left {
                S::link_mut(&mut nodes[parent.0]).left = Some(id);
            } else {
                S::link_mut(&mut nodes[parent.0]).right = Some(id);
            }
        } else {
            self.root = Some(id);
        }
        self.len += 1;
        self.insert_fixup(id, nodes);
        self.first = self.root.map(|root| self.minimum(root, nodes));
        self.last = self.root.map(|root| self.maximum(root, nodes));
        Ok(())
    }

    fn rotate_left<N>(&mut self, x: NodeId, nodes: &mut Slab<N>)
    where
        N: NodeValue,
        S: IndexSpec<N, Link = OrderedLink>,
    {
        let y = S::link(&nodes[x.0])
            .right
            .expect("left rotation needs right child");
        let y_left = S::link(&nodes[y.0]).left;
        S::link_mut(&mut nodes[x.0]).right = y_left;
        if let Some(y_left) = y_left {
            S::link_mut(&mut nodes[y_left.0]).parent = Some(x);
        }
        let x_parent = S::link(&nodes[x.0]).parent;
        S::link_mut(&mut nodes[y.0]).parent = x_parent;
        if let Some(parent) = x_parent {
            if S::link(&nodes[parent.0]).left == Some(x) {
                S::link_mut(&mut nodes[parent.0]).left = Some(y);
            } else {
                S::link_mut(&mut nodes[parent.0]).right = Some(y);
            }
        } else {
            self.root = Some(y);
        }
        S::link_mut(&mut nodes[y.0]).left = Some(x);
        S::link_mut(&mut nodes[x.0]).parent = Some(y);
    }

    fn rotate_right<N>(&mut self, x: NodeId, nodes: &mut Slab<N>)
    where
        N: NodeValue,
        S: IndexSpec<N, Link = OrderedLink>,
    {
        let y = S::link(&nodes[x.0])
            .left
            .expect("right rotation needs left child");
        let y_right = S::link(&nodes[y.0]).right;
        S::link_mut(&mut nodes[x.0]).left = y_right;
        if let Some(y_right) = y_right {
            S::link_mut(&mut nodes[y_right.0]).parent = Some(x);
        }
        let x_parent = S::link(&nodes[x.0]).parent;
        S::link_mut(&mut nodes[y.0]).parent = x_parent;
        if let Some(parent) = x_parent {
            if S::link(&nodes[parent.0]).right == Some(x) {
                S::link_mut(&mut nodes[parent.0]).right = Some(y);
            } else {
                S::link_mut(&mut nodes[parent.0]).left = Some(y);
            }
        } else {
            self.root = Some(y);
        }
        S::link_mut(&mut nodes[y.0]).right = Some(x);
        S::link_mut(&mut nodes[x.0]).parent = Some(y);
    }

    fn insert_fixup<N>(&mut self, mut z: NodeId, nodes: &mut Slab<N>)
    where
        N: NodeValue,
        S: IndexSpec<N, Link = OrderedLink>,
    {
        while self.color(self.parent(Some(z), nodes), nodes) == Color::Red {
            let parent = self.parent(Some(z), nodes).unwrap();
            let grandparent = self.parent(Some(parent), nodes).unwrap();
            if self.left(Some(grandparent), nodes) == Some(parent) {
                let uncle = self.right(Some(grandparent), nodes);
                if self.color(uncle, nodes) == Color::Red {
                    self.set_color(Some(parent), Color::Black, nodes);
                    self.set_color(uncle, Color::Black, nodes);
                    self.set_color(Some(grandparent), Color::Red, nodes);
                    z = grandparent;
                } else {
                    if self.right(Some(parent), nodes) == Some(z) {
                        z = parent;
                        self.rotate_left(z, nodes);
                    }
                    let parent = self.parent(Some(z), nodes).unwrap();
                    let grandparent = self.parent(Some(parent), nodes).unwrap();
                    self.set_color(Some(parent), Color::Black, nodes);
                    self.set_color(Some(grandparent), Color::Red, nodes);
                    self.rotate_right(grandparent, nodes);
                }
            } else {
                let uncle = self.left(Some(grandparent), nodes);
                if self.color(uncle, nodes) == Color::Red {
                    self.set_color(Some(parent), Color::Black, nodes);
                    self.set_color(uncle, Color::Black, nodes);
                    self.set_color(Some(grandparent), Color::Red, nodes);
                    z = grandparent;
                } else {
                    if self.left(Some(parent), nodes) == Some(z) {
                        z = parent;
                        self.rotate_right(z, nodes);
                    }
                    let parent = self.parent(Some(z), nodes).unwrap();
                    let grandparent = self.parent(Some(parent), nodes).unwrap();
                    self.set_color(Some(parent), Color::Black, nodes);
                    self.set_color(Some(grandparent), Color::Red, nodes);
                    self.rotate_left(grandparent, nodes);
                }
            }
        }
        self.set_color(self.root, Color::Black, nodes);
    }

    fn transplant<N>(&mut self, u: NodeId, v: Option<NodeId>, nodes: &mut Slab<N>)
    where
        N: NodeValue,
        S: IndexSpec<N, Link = OrderedLink>,
    {
        let parent = S::link(&nodes[u.0]).parent;
        if let Some(parent) = parent {
            if S::link(&nodes[parent.0]).left == Some(u) {
                S::link_mut(&mut nodes[parent.0]).left = v;
            } else {
                S::link_mut(&mut nodes[parent.0]).right = v;
            }
        } else {
            self.root = v;
        }
        if let Some(v) = v {
            S::link_mut(&mut nodes[v.0]).parent = parent;
        }
    }

    pub fn remove<N>(&mut self, z: NodeId, nodes: &mut Slab<N>)
    where
        N: NodeValue,
        S: IndexSpec<N, Link = OrderedLink>,
    {
        if !S::link(&nodes[z.0]).linked {
            return;
        }
        let mut y = z;
        let mut y_color = S::link(&nodes[y.0]).color;
        let x;
        let x_parent;
        if S::link(&nodes[z.0]).left.is_none() {
            x = S::link(&nodes[z.0]).right;
            x_parent = S::link(&nodes[z.0]).parent;
            self.transplant(z, x, nodes);
        } else if S::link(&nodes[z.0]).right.is_none() {
            x = S::link(&nodes[z.0]).left;
            x_parent = S::link(&nodes[z.0]).parent;
            self.transplant(z, x, nodes);
        } else {
            y = self.minimum(S::link(&nodes[z.0]).right.unwrap(), nodes);
            y_color = S::link(&nodes[y.0]).color;
            x = S::link(&nodes[y.0]).right;
            if S::link(&nodes[y.0]).parent == Some(z) {
                x_parent = Some(y);
                if let Some(x) = x {
                    S::link_mut(&mut nodes[x.0]).parent = Some(y);
                }
            } else {
                x_parent = S::link(&nodes[y.0]).parent;
                self.transplant(y, x, nodes);
                let z_right = S::link(&nodes[z.0]).right;
                S::link_mut(&mut nodes[y.0]).right = z_right;
                if let Some(right) = z_right {
                    S::link_mut(&mut nodes[right.0]).parent = Some(y);
                }
            }
            self.transplant(z, Some(y), nodes);
            let z_left = S::link(&nodes[z.0]).left;
            let z_color = S::link(&nodes[z.0]).color;
            S::link_mut(&mut nodes[y.0]).left = z_left;
            S::link_mut(&mut nodes[y.0]).color = z_color;
            if let Some(left) = z_left {
                S::link_mut(&mut nodes[left.0]).parent = Some(y);
            }
        }
        *S::link_mut(&mut nodes[z.0]) = OrderedLink::default();
        self.len -= 1;
        if y_color == Color::Black {
            self.delete_fixup(x, x_parent, nodes);
        }
        self.first = self.root.map(|root| self.minimum(root, nodes));
        self.last = self.root.map(|root| self.maximum(root, nodes));
    }

    fn delete_fixup<N>(
        &mut self,
        mut x: Option<NodeId>,
        mut parent: Option<NodeId>,
        nodes: &mut Slab<N>,
    ) where
        N: NodeValue,
        S: IndexSpec<N, Link = OrderedLink>,
    {
        while x != self.root && self.color(x, nodes) == Color::Black {
            let Some(parent_id) = parent else {
                break;
            };
            if x == self.left(Some(parent_id), nodes) {
                let mut sibling = self.right(Some(parent_id), nodes);
                if self.color(sibling, nodes) == Color::Red {
                    self.set_color(sibling, Color::Black, nodes);
                    self.set_color(Some(parent_id), Color::Red, nodes);
                    self.rotate_left(parent_id, nodes);
                    sibling = self.right(Some(parent_id), nodes);
                }
                if self.color(self.left(sibling, nodes), nodes) == Color::Black
                    && self.color(self.right(sibling, nodes), nodes) == Color::Black
                {
                    self.set_color(sibling, Color::Red, nodes);
                    x = Some(parent_id);
                    parent = self.parent(x, nodes);
                } else {
                    if self.color(self.right(sibling, nodes), nodes) == Color::Black {
                        self.set_color(self.left(sibling, nodes), Color::Black, nodes);
                        self.set_color(sibling, Color::Red, nodes);
                        if let Some(sibling) = sibling {
                            self.rotate_right(sibling, nodes);
                        }
                        sibling = self.right(Some(parent_id), nodes);
                    }
                    self.set_color(sibling, self.color(Some(parent_id), nodes), nodes);
                    self.set_color(Some(parent_id), Color::Black, nodes);
                    self.set_color(self.right(sibling, nodes), Color::Black, nodes);
                    self.rotate_left(parent_id, nodes);
                    x = self.root;
                    parent = None;
                }
            } else {
                let mut sibling = self.left(Some(parent_id), nodes);
                if self.color(sibling, nodes) == Color::Red {
                    self.set_color(sibling, Color::Black, nodes);
                    self.set_color(Some(parent_id), Color::Red, nodes);
                    self.rotate_right(parent_id, nodes);
                    sibling = self.left(Some(parent_id), nodes);
                }
                if self.color(self.right(sibling, nodes), nodes) == Color::Black
                    && self.color(self.left(sibling, nodes), nodes) == Color::Black
                {
                    self.set_color(sibling, Color::Red, nodes);
                    x = Some(parent_id);
                    parent = self.parent(x, nodes);
                } else {
                    if self.color(self.left(sibling, nodes), nodes) == Color::Black {
                        self.set_color(self.right(sibling, nodes), Color::Black, nodes);
                        self.set_color(sibling, Color::Red, nodes);
                        if let Some(sibling) = sibling {
                            self.rotate_left(sibling, nodes);
                        }
                        sibling = self.left(Some(parent_id), nodes);
                    }
                    self.set_color(sibling, self.color(Some(parent_id), nodes), nodes);
                    self.set_color(Some(parent_id), Color::Black, nodes);
                    self.set_color(self.left(sibling, nodes), Color::Black, nodes);
                    self.rotate_right(parent_id, nodes);
                    x = self.root;
                    parent = None;
                }
            }
        }
        self.set_color(x, Color::Black, nodes);
    }

    pub fn reconcile<N>(&mut self, id: NodeId, nodes: &mut Slab<N>) -> Result<(), NodeId>
    where
        N: NodeValue,
        S: IndexSpec<N, Link = OrderedLink>,
        for<'a> S::Key<'a>: Ord,
    {
        if self.in_place(id, nodes) {
            return Ok(());
        }
        self.remove(id, nodes);
        self.insert(id, nodes)
    }

    fn in_place<N>(&self, id: NodeId, nodes: &Slab<N>) -> bool
    where
        N: NodeValue,
        S: IndexSpec<N, Link = OrderedLink>,
        for<'a> S::Key<'a>: Ord,
    {
        let key = S::key(nodes[id.0].value());
        let previous = self.predecessor(id, nodes);
        let next = self.successor(id, nodes);
        let after_previous = previous.is_none_or(|previous| {
            let previous = S::key(nodes[previous.0].value());
            if UNIQUE {
                previous < key
            } else {
                previous <= key
            }
        });
        let before_next = next.is_none_or(|next| {
            let next = S::key(nodes[next.0].value());
            if UNIQUE {
                key < next
            } else {
                key <= next
            }
        });
        after_previous && before_next
    }

    pub fn validate<N>(&self, nodes: &Slab<N>) -> Result<(), String>
    where
        N: NodeValue,
        S: IndexSpec<N, Link = OrderedLink>,
        for<'a> S::Key<'a>: Ord,
    {
        if self.color(self.root, nodes) != Color::Black {
            return Err(format!("{} root is not black", S::NAME));
        }
        let mut seen = vec![false; nodes.capacity().max(1)];
        let (_, count) = self.validate_subtree(self.root, None, None, nodes, &mut seen)?;
        if count != self.len {
            return Err(format!("{} count does not match its tree", S::NAME));
        }
        let ids = self.ids(nodes);
        if ids.first().copied() != self.first || ids.last().copied() != self.last {
            return Err(format!("{} first/last links are stale", S::NAME));
        }
        for pair in ids.windows(2) {
            let left = S::key(nodes[pair[0].0].value());
            let right = S::key(nodes[pair[1].0].value());
            if if UNIQUE { left >= right } else { left > right } {
                return Err(format!("{} tree is out of order", S::NAME));
            }
        }
        for (id, node) in nodes.iter() {
            if S::link(node).linked != seen.get(id).copied().unwrap_or(false) {
                return Err(format!("{} membership does not match the arena", S::NAME));
            }
        }
        Ok(())
    }

    fn validate_subtree<N>(
        &self,
        id: Option<NodeId>,
        min: Option<NodeId>,
        max: Option<NodeId>,
        nodes: &Slab<N>,
        seen: &mut [bool],
    ) -> Result<(usize, usize), String>
    where
        N: NodeValue,
        S: IndexSpec<N, Link = OrderedLink>,
        for<'a> S::Key<'a>: Ord,
    {
        let Some(id) = id else {
            return Ok((1, 0));
        };
        if id.0 >= seen.len() || seen[id.0] {
            return Err(format!("{} contains a cycle or duplicate node", S::NAME));
        }
        seen[id.0] = true;
        let node = nodes
            .get(id.0)
            .ok_or_else(|| format!("{} points outside the arena", S::NAME))?;
        let link = S::link(node);
        if !link.linked {
            return Err(format!("{} contains an unlinked node", S::NAME));
        }
        let key = S::key(node.value());
        if min.is_some_and(|min| {
            let min = S::key(nodes[min.0].value());
            if UNIQUE {
                key <= min
            } else {
                key < min
            }
        }) || max.is_some_and(|max| {
            let max = S::key(nodes[max.0].value());
            if UNIQUE {
                key >= max
            } else {
                key > max
            }
        }) {
            return Err(format!("{} violates tree ordering", S::NAME));
        }
        for child in [link.left, link.right].into_iter().flatten() {
            if S::link(&nodes[child.0]).parent != Some(id) {
                return Err(format!("{} has inconsistent parent links", S::NAME));
            }
        }
        if link.color == Color::Red
            && (self.color(link.left, nodes) == Color::Red
                || self.color(link.right, nodes) == Color::Red)
        {
            return Err(format!("{} has adjacent red nodes", S::NAME));
        }
        let (left_black, left_count) =
            self.validate_subtree(link.left, min, Some(id), nodes, seen)?;
        let (right_black, right_count) =
            self.validate_subtree(link.right, Some(id), max, nodes, seen)?;
        if left_black != right_black {
            return Err(format!("{} has unequal black heights", S::NAME));
        }
        Ok((
            left_black + usize::from(link.color == Color::Black),
            left_count + right_count + 1,
        ))
    }
}

macro_rules! impl_hashed_kind {
    ($kind:ty, $unique:literal) => {
        impl<N, S> IndexKind<N, S> for $kind
        where
            N: NodeValue,
            S: IndexSpec<N, Link = HashLink> + 'static,
            for<'a> S::Key<'a>: Eq + Hash,
        {
            type Index = HashedIndex<S, $unique>;
            type Ids<'a>
                = HashIds<'a, N, S, $unique>
            where
                N: 'a,
                S: 'a,
                Self::Index: 'a;
            type EqualIds<'a>
                = HashEqualIds<'a, N, S, $unique>
            where
                N: 'a,
                S: 'a,
                Self::Index: 'a;

            fn len(index: &Self::Index) -> usize {
                index.len()
            }

            fn clear(index: &mut Self::Index) {
                index.clear();
            }

            fn reserve_for_insert(index: &mut Self::Index, nodes: &mut Slab<N>) {
                index.reserve_for_insert(nodes);
            }

            fn iter_ids<'a>(index: &'a Self::Index, nodes: &'a Slab<N>) -> Self::Ids<'a> {
                index.iter_ids(nodes)
            }

            fn insert(
                index: &mut Self::Index,
                id: NodeId,
                nodes: &mut Slab<N>,
            ) -> Result<(), NodeId> {
                index.insert(id, nodes)
            }

            fn remove(index: &mut Self::Index, id: NodeId, nodes: &mut Slab<N>) {
                index.remove(id, nodes);
            }

            fn reconcile(
                index: &mut Self::Index,
                id: NodeId,
                nodes: &mut Slab<N>,
            ) -> Result<(), NodeId> {
                index.reconcile(id, nodes)
            }

            fn validate(index: &Self::Index, nodes: &Slab<N>) -> Result<(), String> {
                index.validate(nodes)
            }
        }

        impl<N, S, Q> QueryIndexKind<N, S, Q> for $kind
        where
            N: NodeValue,
            S: IndexSpec<N, Link = HashLink> + 'static,
            Q: Eq + Hash + ?Sized,
            for<'a> S::Key<'a>: Eq + Hash + Equivalent<Q>,
        {
            fn find(index: &Self::Index, key: &Q, nodes: &Slab<N>) -> Option<NodeId> {
                index.find(key, nodes)
            }

            fn equal_ids(index: &Self::Index, key: &Q, nodes: &Slab<N>) -> Vec<NodeId> {
                index.equal_ids(key, nodes)
            }

            fn equal_iter_ids<'a>(
                index: &'a Self::Index,
                key: &Q,
                nodes: &'a Slab<N>,
            ) -> Self::EqualIds<'a> {
                index.equal_iter_ids(key, nodes)
            }
        }
    };
}

macro_rules! impl_ordered_kind {
    ($kind:ty, $unique:literal) => {
        impl<N, S> IndexKind<N, S> for $kind
        where
            N: NodeValue,
            S: IndexSpec<N, Link = OrderedLink> + 'static,
            for<'a> S::Key<'a>: Ord,
        {
            type Index = OrderedIndex<S, $unique>;
            type Ids<'a>
                = OrderedIds<'a, N, S, $unique>
            where
                N: 'a,
                S: 'a,
                Self::Index: 'a;
            type EqualIds<'a>
                = OrderedRangeIds<'a, N, S, $unique>
            where
                N: 'a,
                S: 'a,
                Self::Index: 'a;

            fn len(index: &Self::Index) -> usize {
                index.len()
            }

            fn clear(index: &mut Self::Index) {
                index.clear();
            }

            fn reserve_for_insert(_index: &mut Self::Index, _nodes: &mut Slab<N>) {}

            fn iter_ids<'a>(index: &'a Self::Index, nodes: &'a Slab<N>) -> Self::Ids<'a> {
                index.iter_ids(nodes)
            }

            fn insert(
                index: &mut Self::Index,
                id: NodeId,
                nodes: &mut Slab<N>,
            ) -> Result<(), NodeId> {
                index.insert(id, nodes)
            }

            fn remove(index: &mut Self::Index, id: NodeId, nodes: &mut Slab<N>) {
                index.remove(id, nodes);
            }

            fn reconcile(
                index: &mut Self::Index,
                id: NodeId,
                nodes: &mut Slab<N>,
            ) -> Result<(), NodeId> {
                index.reconcile(id, nodes)
            }

            fn validate(index: &Self::Index, nodes: &Slab<N>) -> Result<(), String> {
                index.validate(nodes)
            }
        }

        impl<N, S, Q> QueryIndexKind<N, S, Q> for $kind
        where
            N: NodeValue,
            S: IndexSpec<N, Link = OrderedLink> + 'static,
            Q: Ord + ?Sized,
            for<'a> S::Key<'a>: Ord + Compare<Q>,
        {
            fn find(index: &Self::Index, key: &Q, nodes: &Slab<N>) -> Option<NodeId> {
                index.find(key, nodes)
            }

            fn equal_ids(index: &Self::Index, key: &Q, nodes: &Slab<N>) -> Vec<NodeId> {
                index.equal_ids(key, nodes)
            }

            fn equal_iter_ids<'a>(
                index: &'a Self::Index,
                key: &Q,
                nodes: &'a Slab<N>,
            ) -> Self::EqualIds<'a> {
                index.equal_iter_ids(key, nodes)
            }
        }

        impl<N, S> OrderedIndexKind<N, S> for $kind
        where
            N: NodeValue,
            S: IndexSpec<N, Link = OrderedLink> + 'static,
            for<'a> S::Key<'a>: Ord,
        {
            type RangeIds<'a>
                = OrderedRangeIds<'a, N, S, $unique>
            where
                N: 'a,
                S: 'a,
                Self::Index: 'a;

            fn range_iter_ids<'a, Q, R>(
                index: &'a Self::Index,
                range: R,
                nodes: &'a Slab<N>,
            ) -> Self::RangeIds<'a>
            where
                R: RangeBounds<Q>,
                Q: Ord + ?Sized,
                for<'key> S::Key<'key>: Compare<Q>,
            {
                index.range_iter_ids(range, nodes)
            }
        }
    };
}

impl_hashed_kind!(HashedUnique, true);
impl_hashed_kind!(HashedNonUnique, false);
impl_ordered_kind!(OrderedUnique, true);
impl_ordered_kind!(OrderedNonUnique, false);
