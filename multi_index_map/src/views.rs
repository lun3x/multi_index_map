use std::ops::RangeBounds;

/// Common read-only capabilities shared by every index view.
///
/// Trait methods use the index's exact key type. Generated inherent methods can
/// additionally accept richer borrowed-query types without complicating these
/// capability traits.
pub trait IndexView {
    type Value;
    type Key: ?Sized;
    type Iter<'a>: Iterator<Item = &'a Self::Value>
    where
        Self: 'a,
        Self::Value: 'a;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn iter(&self) -> Self::Iter<'_>;
}

/// Read capabilities provided by unique indices.
pub trait UniqueView: IndexView {
    fn get(&self, key: &Self::Key) -> Option<&Self::Value>;

    fn contains_key(&self, key: &Self::Key) -> bool {
        self.get(key).is_some()
    }
}

/// Read capabilities provided by non-unique indices.
pub trait NonUniqueView: IndexView {
    type EqualRange<'a>: Iterator<Item = &'a Self::Value>
    where
        Self: 'a,
        Self::Value: 'a;

    fn equal_range(&self, key: &Self::Key) -> Self::EqualRange<'_>;
}

/// Additional sorted traversal provided by ordered indices.
pub trait OrderedView: IndexView {
    type Range<'a>: DoubleEndedIterator<Item = &'a Self::Value>
    where
        Self: 'a,
        Self::Value: 'a;

    fn range<R>(&self, range: R) -> Self::Range<'_>
    where
        R: RangeBounds<Self::Key>;
}

/// Non-indexed-field mutation shared by every mutable index view.
///
/// The selected index determines traversal order. Ordered indices visit values
/// in sorted order, while hashed index order is unspecified. The traversal is
/// snapshotted before the first update, so each original element is visited
/// exactly once. If the callback panics, completed and partial field updates
/// remain, but index invariants are unaffected.
pub trait IndexViewMut: IndexView {
    type Update<'a>;

    fn update_each<F>(&mut self, f: F) -> usize
    where
        F: for<'a> FnMut(Self::Update<'a>);
}

/// Mutation capabilities provided by unique indices.
///
/// A mutable unique view also implements its corresponding read capabilities.
pub trait UniqueViewMut: UniqueView + IndexViewMut {
    type Conflict;

    fn remove(&mut self, key: &Self::Key) -> Option<Self::Value>;

    fn replace(
        &mut self,
        key: &Self::Key,
        replacement: Self::Value,
    ) -> Result<Option<Self::Value>, Self::Conflict>;

    fn modify<F>(&mut self, key: &Self::Key, f: F) -> Result<Option<&Self::Value>, Self::Conflict>
    where
        F: FnOnce(&mut Self::Value);

    fn update<F>(&mut self, key: &Self::Key, f: F) -> Option<&Self::Value>
    where
        F: for<'a> FnOnce(Self::Update<'a>);
}

/// Mutation capabilities provided by non-unique indices.
///
/// Batch methods snapshot the original equal range before making changes.
pub trait NonUniqueViewMut: NonUniqueView + IndexViewMut {
    type ModifyAllResult;

    fn remove_all(&mut self, key: &Self::Key) -> Vec<Self::Value>;

    fn modify_all<F>(&mut self, key: &Self::Key, f: F) -> Self::ModifyAllResult
    where
        F: FnMut(&mut Self::Value);

    fn update_all<F>(&mut self, key: &Self::Key, f: F) -> usize
    where
        F: for<'a> FnMut(Self::Update<'a>);
}
