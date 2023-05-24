use criterion::{Criterion, black_box, criterion_group, criterion_main};
use multi_index_map::MultiIndexMap;


#[derive(Hash, PartialEq, Eq, Clone, Debug)]
struct TestNonPrimitiveType(u32);

#[derive(MultiIndexMap, Debug, Clone)]
pub struct TestElementWithOnlyIndexedFields {
    #[multi_index(hashed_unique)]
    field_hashed_unique: TestNonPrimitiveType,
    #[multi_index(hashed_non_unique)]
    field_hashed_non_unique: TestNonPrimitiveType,
    #[multi_index(ordered_unique)]
    field_ordered_unique: u32,
    #[multi_index(ordered_non_unique)]
    field_ordered_non_unique: u32,
}

const BENCH_SIZES: &'static[u32] = &[100u32, 1000u32, 10000u32, 100000u32];

fn insert_benchmark(c: &mut Criterion) {
    fn inner(c: &mut Criterion, n: u32) {
        let mut map = black_box(MultiIndexTestElementWithOnlyIndexedFieldsMap::default());
        c.bench_function(&format!("insert_bench_{}", n), |b|{b.iter(|| {
            for i in 0..n {
                map.insert(TestElementWithOnlyIndexedFields { 
                    field_hashed_unique: TestNonPrimitiveType(i), 
                    field_hashed_non_unique: TestNonPrimitiveType(42), 
                    field_ordered_unique: i, 
                    field_ordered_non_unique: i/5 
                });
                map.clear();
            }
        })});
    }

    for n in BENCH_SIZES {
        inner(c, *n);
    }
}

fn delete_by_hashed_unique_key_benchmark(c: &mut Criterion) {
    fn inner(c: &mut Criterion, n: u32) {
        let mut map = black_box(MultiIndexTestElementWithOnlyIndexedFieldsMap::default());
        for i in 0..n {
            map.insert(TestElementWithOnlyIndexedFields { 
                field_hashed_unique: TestNonPrimitiveType(i), 
                field_hashed_non_unique: TestNonPrimitiveType(42), 
                field_ordered_unique: i, 
                field_ordered_non_unique: i/5 
            });}
        c.bench_function(&format!("delete_hashed_unique_key_bench_{}", n), |b|{b.iter(||{
            let mut map_clone = map.clone();
            for i in 0..n {
                map_clone.remove_by_field_hashed_unique(&TestNonPrimitiveType(i));
            }
        })});
    }

    for n in BENCH_SIZES {
        inner(c, *n);
    } 
}

fn delete_by_hashed_non_unique_key_benchmark(c: &mut Criterion) {
    fn inner(c: &mut Criterion, n: u32) {
        let mut map = black_box(MultiIndexTestElementWithOnlyIndexedFieldsMap::default());
        for i in 0..n {
            map.insert(TestElementWithOnlyIndexedFields { 
                field_hashed_unique: TestNonPrimitiveType(i), 
                field_hashed_non_unique: TestNonPrimitiveType(42+i%5), 
                field_ordered_unique: i, 
                field_ordered_non_unique: i/5 
            });}
        c.bench_function(&format!("delete_hashed_non_unique_key_bench_{}", n), |b|{b.iter(||{
            let mut map_clone = map.clone();
            for i in 0..10 {
                map_clone.remove_by_field_hashed_non_unique(&TestNonPrimitiveType(42+i%5));
            }
        })});
    }

    for n in BENCH_SIZES {
        inner(c, *n);
    } 
}

fn delete_by_ordered_unique_key_benchmark(c: &mut Criterion) {
    fn inner(c: &mut Criterion, n: u32) {
        let mut map = black_box(MultiIndexTestElementWithOnlyIndexedFieldsMap::default());
        for i in 0..n {
            map.insert(TestElementWithOnlyIndexedFields { 
                field_hashed_unique: TestNonPrimitiveType(i), 
                field_hashed_non_unique: TestNonPrimitiveType(42+i%5), 
                field_ordered_unique: i, 
                field_ordered_non_unique: i/5 
            });}
        c.bench_function(&format!("delete_ordered_unique_key_bench_{}", n), |b|{b.iter(||{
            let mut map_clone = map.clone();
            for i in 0..n {
                map_clone.remove_by_field_ordered_unique(&i);
            }
        })});
    }

    for n in BENCH_SIZES {
        inner(c, *n);
    } 
}

fn delete_by_ordered_non_unique_key_benchmark(c: &mut Criterion) {
    fn inner(c: &mut Criterion, n: u32) {
        let mut map = black_box(MultiIndexTestElementWithOnlyIndexedFieldsMap::default());
        for i in 0..n {
            map.insert(TestElementWithOnlyIndexedFields { 
                field_hashed_unique: TestNonPrimitiveType(i), 
                field_hashed_non_unique: TestNonPrimitiveType(42+i%5), 
                field_ordered_unique: i, 
                field_ordered_non_unique: i/5 
            });}
        c.bench_function(&format!("delete_ordered_non_unique_key_bench_{}", n), |b|{b.iter(||{
            let mut map_clone = map.clone();
            for i in 0..n {
                map_clone.remove_by_field_ordered_non_unique(&(i/5));
            }
        })});
    }
    
    for n in BENCH_SIZES {
        inner(c, *n);
    } 
}

fn modify_hashed_unique_key_by_hashed_unique_key_benchmark(c: &mut Criterion) {
    fn inner(c: &mut Criterion, n: u32) {
        let mut map = black_box(MultiIndexTestElementWithOnlyIndexedFieldsMap::default());
        for i in 0..n {
            map.insert(TestElementWithOnlyIndexedFields { 
                field_hashed_unique: TestNonPrimitiveType(i), 
                field_hashed_non_unique: TestNonPrimitiveType(42+i%5), 
                field_ordered_unique: i, 
                field_ordered_non_unique: i/5 
            });}
        c.bench_function(&format!("modify_hashed_unique_key_by_hashed_unique_key_bench_{}", n), |b|{b.iter(||{
            let mut map_clone = map.clone();
            for i in 0..n {
                map_clone.modify_by_field_hashed_unique(&TestNonPrimitiveType(i), |e| {
                    e.field_hashed_unique = TestNonPrimitiveType(e.field_hashed_unique.0 + n);
                });
            }
        })});
    }

    for n in BENCH_SIZES {
        inner(c, *n);
    } 
}

fn modify_hashed_non_unique_key_by_hashed_unique_key_benchmark(c: &mut Criterion) {
    fn inner(c: &mut Criterion, n: u32) {
        let mut map = black_box(MultiIndexTestElementWithOnlyIndexedFieldsMap::default());
        for i in 0..n {
            map.insert(TestElementWithOnlyIndexedFields { 
                field_hashed_unique: TestNonPrimitiveType(i), 
                field_hashed_non_unique: TestNonPrimitiveType(42+i%5), 
                field_ordered_unique: i, 
                field_ordered_non_unique: i/5 
            });}
        c.bench_function(&format!("modify_hashed_non_unique_key_by_hashed_unique_key_bench_{}", n), |b|{b.iter(||{
            let mut map_clone = map.clone();
            for i in 0..n {
                map_clone.modify_by_field_hashed_unique(&TestNonPrimitiveType(i), |e| {
                    e.field_hashed_non_unique = TestNonPrimitiveType(e.field_hashed_non_unique.0 + 1);
                });
            }
        })});
    }

    for n in BENCH_SIZES {
        inner(c, *n);
    } 
}

fn modify_ordered_unique_key_by_hashed_unique_key_benchmark(c: &mut Criterion) {
    fn inner(c: &mut Criterion, n: u32) {
        let mut map = black_box(MultiIndexTestElementWithOnlyIndexedFieldsMap::default());
        for i in 0..n {
            map.insert(TestElementWithOnlyIndexedFields { 
                field_hashed_unique: TestNonPrimitiveType(i), 
                field_hashed_non_unique: TestNonPrimitiveType(42+i%5), 
                field_ordered_unique: i, 
                field_ordered_non_unique: i/5 
            });}
        c.bench_function(&format!("modify_ordered_unique_key_by_hashed_unique_key_bench_{}", n), |b|{b.iter(||{
            let mut map_clone = map.clone();
            for i in 0..n {
                map_clone.modify_by_field_hashed_unique(&TestNonPrimitiveType(i), |e| {
                    e.field_ordered_unique = e.field_ordered_unique + n;
                });
            }
        })});
    }

    for n in BENCH_SIZES {
        inner(c, *n);
    } 
}

fn modify_ordered_non_unique_key_by_hashed_unique_key_benchmark(c: &mut Criterion) {
    fn inner(c: &mut Criterion, n: u32) {
        let mut map = black_box(MultiIndexTestElementWithOnlyIndexedFieldsMap::default());
        for i in 0..n {
            map.insert(TestElementWithOnlyIndexedFields { 
                field_hashed_unique: TestNonPrimitiveType(i), 
                field_hashed_non_unique: TestNonPrimitiveType(42+i%5), 
                field_ordered_unique: i, 
                field_ordered_non_unique: i/5 
            });}
        c.bench_function(&format!("modify_ordered_unique_key_by_hashed_unique_key_bench_{}", n), |b|{b.iter(||{
            let mut map_clone = map.clone();
            for i in 0..n {
                map_clone.modify_by_field_hashed_unique(&TestNonPrimitiveType(i), |e| {
                    e.field_ordered_non_unique = e.field_ordered_non_unique + 1;
                });
            }
        })});
    }

    for n in BENCH_SIZES {
        inner(c, *n);
    } 
}

fn modify_hashed_unique_key_by_ordered_unique_key_benchmark(c: &mut Criterion) {
    fn inner(c: &mut Criterion, n: u32) {
        let mut map = black_box(MultiIndexTestElementWithOnlyIndexedFieldsMap::default());
        for i in 0..n {
            map.insert(TestElementWithOnlyIndexedFields { 
                field_hashed_unique: TestNonPrimitiveType(i), 
                field_hashed_non_unique: TestNonPrimitiveType(42+i%5), 
                field_ordered_unique: i, 
                field_ordered_non_unique: i/5 
            });}
        c.bench_function(&format!("modify_hashed_unique_key_by_ordered_unique_key_bench_{}", n), |b|{b.iter(||{
            let mut map_clone = map.clone();
            for i in 0..n {
                map_clone.modify_by_field_ordered_unique(&i, |e| {
                    e.field_hashed_unique = TestNonPrimitiveType(e.field_hashed_unique.0 + n);
                });
            }
        })});
    }

    for n in BENCH_SIZES {
        inner(c, *n);
    } 
}

fn modify_hashed_non_unique_key_by_ordered_unique_key_benchmark(c: &mut Criterion) {
    fn inner(c: &mut Criterion, n: u32) {
        let mut map = black_box(MultiIndexTestElementWithOnlyIndexedFieldsMap::default());
        for i in 0..n {
            map.insert(TestElementWithOnlyIndexedFields { 
                field_hashed_unique: TestNonPrimitiveType(i), 
                field_hashed_non_unique: TestNonPrimitiveType(42+i%5), 
                field_ordered_unique: i, 
                field_ordered_non_unique: i/5 
            });}
        c.bench_function(&format!("modify_hashed_non_unique_key_by_ordered_unique_key_bench_{}", n), |b|{b.iter(||{
            let mut map_clone = map.clone();
            for i in 0..n {
                map_clone.modify_by_field_ordered_unique(&i, |e| {
                    e.field_hashed_non_unique = TestNonPrimitiveType(e.field_hashed_non_unique.0 + 1);
                });
            }
        })});
    }

    for n in BENCH_SIZES {
        inner(c, *n);
    } 
}

fn modify_ordered_unique_key_by_ordered_unique_key_benchmark(c: &mut Criterion) {
    fn inner(c: &mut Criterion, n: u32) {
        let mut map = black_box(MultiIndexTestElementWithOnlyIndexedFieldsMap::default());
        for i in 0..n {
            map.insert(TestElementWithOnlyIndexedFields { 
                field_hashed_unique: TestNonPrimitiveType(i), 
                field_hashed_non_unique: TestNonPrimitiveType(42+i%5), 
                field_ordered_unique: i, 
                field_ordered_non_unique: i/5 
            });}
        c.bench_function(&format!("modify_ordered_unique_key_by_ordered_unique_key_bench_{}", n), |b|{b.iter(||{
            let mut map_clone = map.clone();
            for i in 0..n {
                map_clone.modify_by_field_ordered_unique(&i, |e| {
                    e.field_ordered_unique = e.field_ordered_unique + n;
                });
            }
        })});
    }

    for n in BENCH_SIZES {
        inner(c, *n);
    } 
}

fn modify_ordered_non_unique_key_by_ordered_unique_key_benchmark(c: &mut Criterion) {
    fn inner(c: &mut Criterion, n: u32) {
        let mut map = black_box(MultiIndexTestElementWithOnlyIndexedFieldsMap::default());
        for i in 0..n {
            map.insert(TestElementWithOnlyIndexedFields { 
                field_hashed_unique: TestNonPrimitiveType(i), 
                field_hashed_non_unique: TestNonPrimitiveType(42+i%5), 
                field_ordered_unique: i, 
                field_ordered_non_unique: i/5 
            });}
        c.bench_function(&format!("modify_ordered_unique_key_by_ordered_unique_key_bench_{}", n), |b|{b.iter(||{
            let mut map_clone = map.clone();
            for i in 0..n {
                map_clone.modify_by_field_ordered_unique(&i, |e| {
                    e.field_ordered_non_unique = e.field_ordered_non_unique + 1;
                });
            }
        })});
    }

    for n in BENCH_SIZES {
        inner(c, *n);
    } 
}

fn unique_key_iter_benchmark(c: &mut Criterion) {
    fn inner(c: &mut Criterion, n: u32) {
        let mut map = black_box(MultiIndexTestElementWithOnlyIndexedFieldsMap::default());
        for i in 0..n {
            map.insert(TestElementWithOnlyIndexedFields { 
                field_hashed_unique: TestNonPrimitiveType(i), 
                field_hashed_non_unique: TestNonPrimitiveType(42), 
                field_ordered_unique: i, 
                field_ordered_non_unique: i/5 
            });
            map.clear();
        }
        c.bench_function(&format!("unique_key_iter_bench_{}", n), |b|{b.iter(|| {
            for _ in map.iter_by_field_hashed_unique() {()}
            for _ in map.iter_by_field_ordered_unique() {()}
        })}); 
    }

    for n in BENCH_SIZES {
        inner(c, *n);
    } 
}

fn non_unique_key_iter_benchmark(c: &mut Criterion) {
    fn inner(c: &mut Criterion, n: u32) {
        let mut map = black_box(MultiIndexTestElementWithOnlyIndexedFieldsMap::default());
        for i in 0..n {
            map.insert(TestElementWithOnlyIndexedFields { 
                field_hashed_unique: TestNonPrimitiveType(i), 
                field_hashed_non_unique: TestNonPrimitiveType(42), 
                field_ordered_unique: i, 
                field_ordered_non_unique: i/5 
            });
            map.clear();
        }
        c.bench_function(&format!("non_unique_key_iter_bench_{}", n), |b|{b.iter(|| {
            for _ in map.iter_by_field_hashed_non_unique() {()}
            for _ in map.iter_by_field_ordered_non_unique() {()}
        })}); 
    }
    
    for n in BENCH_SIZES {
        inner(c, *n);
    } 
}

criterion_group!(
    benches, 
    insert_benchmark, 
    delete_by_hashed_non_unique_key_benchmark, 
    delete_by_hashed_unique_key_benchmark, 
    delete_by_ordered_non_unique_key_benchmark, 
    delete_by_ordered_unique_key_benchmark,
    modify_hashed_unique_key_by_hashed_unique_key_benchmark,
    modify_hashed_non_unique_key_by_hashed_unique_key_benchmark,
    modify_ordered_unique_key_by_hashed_unique_key_benchmark,
    modify_ordered_non_unique_key_by_hashed_unique_key_benchmark,
    modify_hashed_unique_key_by_ordered_unique_key_benchmark,
    modify_hashed_non_unique_key_by_ordered_unique_key_benchmark,
    modify_ordered_unique_key_by_ordered_unique_key_benchmark,
    modify_ordered_non_unique_key_by_ordered_unique_key_benchmark,
    unique_key_iter_benchmark,
    non_unique_key_iter_benchmark,
);

criterion_main!(benches);
