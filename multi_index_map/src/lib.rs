pub use multi_index_map_derive::MultiIndexMap;
pub use multi_index_map_derive2::MultiIndexSelector;
pub use multi_index_map_derive2::MultiIndexMap as MultiIndexMap2;
pub use views::{
    IndexView, NonUniqueView, NonUniqueViewMut, OrderedView, UniqueView, UniqueViewMut,
};

#[doc(hidden)]
pub mod __private {
    pub use crate::private::*;
    pub use slab::Slab;
}

mod private;
pub mod views;

/// A type-level marker describing one index used by an experimental
/// [`MultiIndexMap2`] map.
pub trait MultiIndexSelector {
    #[doc(hidden)]
    type Kind: __private::IndexCategory;

    #[doc(hidden)]
    const NAME: &'static str;
}

#[derive(Debug, PartialEq, Eq)]
pub struct Conflict<T> {
    pub index: &'static str,
    pub value: T,
}

impl<T> core::fmt::Display for Conflict<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "unique index '{}' rejected the value", self.index)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ModifyAllResult<T> {
    pub modified: usize,
    pub removed: Vec<Conflict<T>>,
}

impl<T> Default for ModifyAllResult<T> {
    fn default() -> Self {
        Self {
            modified: 0,
            removed: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct UniquenessError<T>(pub T);

impl<T> core::fmt::Display for UniquenessError<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(
            f,
            "Unable to insert element, uniqueness constraint violated"
        )
    }
}

impl<T> core::fmt::Debug for UniquenessError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("UniquenessViolated").finish()
    }
}

#[doc(hidden)]
#[cfg(feature = "rustc-hash")]
pub use rustc_hash;
#[doc(hidden)]
pub use slab;
