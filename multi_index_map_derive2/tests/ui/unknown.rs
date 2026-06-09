use multi_index_map_derive2::MultiIndexMap;

#[derive(MultiIndexMap)]
struct Unknown {
    #[multi_index(something_else)]
    key: u64,
}

fn main() {}
