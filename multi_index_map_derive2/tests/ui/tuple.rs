use multi_index_map_derive2::MultiIndexMap;

#[derive(MultiIndexMap)]
struct Tuple(#[multi_index(ByKey)] u64);

fn main() {}
