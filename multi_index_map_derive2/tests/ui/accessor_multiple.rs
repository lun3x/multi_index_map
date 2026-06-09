use multi_index_map_derive2::MultiIndexAccessor;

#[derive(MultiIndexAccessor)]
#[multi_index(hashed_unique)]
#[multi_index(ordered_unique)]
struct Multiple;

fn main() {}
