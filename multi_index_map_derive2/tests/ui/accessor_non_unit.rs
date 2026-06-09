use multi_index_map_derive2::MultiIndexAccessor;

#[derive(MultiIndexAccessor)]
#[multi_index(hashed_unique)]
struct NonUnit(u64);

fn main() {}
