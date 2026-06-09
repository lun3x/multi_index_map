use multi_index_map_derive2::MultiIndexMap;

#[derive(MultiIndexMap)]
struct Tuple(#[multi_index(hashed_unique)] u64);

fn main() {}
