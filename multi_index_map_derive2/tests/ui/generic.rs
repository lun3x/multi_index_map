use multi_index_map_derive2::MultiIndexMap;

#[derive(MultiIndexMap)]
struct Generic<T> {
    #[multi_index(hashed_unique)]
    key: T,
}

fn main() {}
