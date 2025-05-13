pub use multi_index_map_derive::MultiIndexMap;

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
