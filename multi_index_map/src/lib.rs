pub use multi_index_map_derive::MultiIndexMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MultiIndexMapError {
    UniquenessViolated,
}

#[doc(hidden)]
pub use rustc_hash;
#[doc(hidden)]
pub use slab;
