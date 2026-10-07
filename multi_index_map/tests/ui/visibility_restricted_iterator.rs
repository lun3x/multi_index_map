// An iterator for a restricted element must stay inside the allowed scope,
// even when its indexed field is public.

use multi_index_map::MultiIndexMap;

mod scope {
    pub mod elements {
        use super::super::MultiIndexMap;

        #[derive(MultiIndexMap, Debug)]
        pub(in crate::scope) struct Element {
            #[multi_index(ordered_non_unique)]
            pub id: u32,
        }
    }
}

fn main() {
    let _: Option<scope::elements::MultiIndexElementMapIdIter<'_>> = None;
}
