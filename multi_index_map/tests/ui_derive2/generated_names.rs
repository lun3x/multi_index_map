use multi_index_map::{MultiIndexAccessor, MultiIndexMap2};

#[derive(MultiIndexAccessor)]
#[multi_index(hashed_unique)]
struct ById;

#[derive(MultiIndexMap2)]
struct Record {
    #[multi_index(ById)]
    id: u64,
    note: String,
}

fn selector<I: MultiIndexRecordMapIndex>() {}

fn main() {
    let _: Option<MultiIndexRecordMapUpdate<'static>> = None;
    let _: Option<__MultiIndexRecordMapNode> = None;
    let _: Option<__MultiIndexRecordMapIndex0Spec> = None;
    let _: Option<__MultiIndexRecordMapIndex0View<'static, ()>> = None;
    let _: Option<__MultiIndexRecordMapIndex0Iter<'static, ()>> = None;
    selector::<ById>();
}
