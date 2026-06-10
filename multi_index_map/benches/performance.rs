use criterion::{criterion_group, criterion_main, Criterion};
use multi_index_map::{MultiIndexMap2, MultiIndexSelector};
use std::hint::black_box;

#[derive(Hash, PartialEq, Eq, Clone, Debug, PartialOrd, Ord)]
pub struct TestNonPrimitiveType(u32);

#[derive(MultiIndexSelector)]
#[multi_index(hashed_unique)]
pub struct ByFieldHashedUnique;

#[derive(MultiIndexSelector)]
#[multi_index(hashed_non_unique)]
pub struct ByFieldHashedNonUnique;

#[derive(MultiIndexSelector)]
#[multi_index(ordered_unique)]
pub struct ByFieldOrderedUnique;

#[derive(MultiIndexSelector)]
#[multi_index(ordered_non_unique)]
pub struct ByFieldOrderedNonUnique;

#[allow(clippy::struct_field_names)]
#[derive(MultiIndexMap2, Debug, Clone)]
#[multi_index_derive(Clone, Debug)]
pub struct TestElementWithOnlyIndexedFields {
    #[multi_index(by(ByFieldHashedUnique))]
    field_hashed_unique: TestNonPrimitiveType,
    #[multi_index(by(ByFieldHashedNonUnique))]
    field_hashed_non_unique: TestNonPrimitiveType,
    #[multi_index(by(ByFieldOrderedUnique))]
    field_ordered_unique: u32,
    #[multi_index(by(ByFieldOrderedNonUnique))]
    field_ordered_non_unique: u32,
}

const BENCH_SIZES: &[u32] = &[100u32];

fn insert_benchmark(c: &mut Criterion) {
    fn inner(c: &mut Criterion, n: u32) {
        let mut map = black_box(MultiIndexTestElementWithOnlyIndexedFieldsMap::default());
        c.bench_function(&format!("insert_bench_{n}"), |b| {
            b.iter(|| {
                for i in 0..n {
                    map.insert(black_box(TestElementWithOnlyIndexedFields {
                        field_hashed_unique: TestNonPrimitiveType(i),
                        field_hashed_non_unique: TestNonPrimitiveType(42),
                        field_ordered_unique: i,
                        field_ordered_non_unique: i / 5,
                    }));
                    map.clear();
                }
            });
        });
    }

    for n in BENCH_SIZES {
        inner(c, *n);
    }
}

fn delete_by_hashed_unique_key_benchmark(c: &mut Criterion) {
    fn inner(c: &mut Criterion, n: u32) {
        let mut map = black_box(MultiIndexTestElementWithOnlyIndexedFieldsMap::default());
        for i in 0..n {
            map.insert(black_box(TestElementWithOnlyIndexedFields {
                field_hashed_unique: TestNonPrimitiveType(i),
                field_hashed_non_unique: TestNonPrimitiveType(42),
                field_ordered_unique: i,
                field_ordered_non_unique: i / 5,
            }));
        }
        c.bench_function(&format!("delete_hashed_unique_key_bench_{n}"), |b| {
            b.iter(|| {
                let mut map_clone = black_box(map.clone());
                for i in 0..n {
                    black_box(
                        map_clone
                            .by_mut::<ByFieldHashedUnique>()
                            .remove(black_box(&TestNonPrimitiveType(i))),
                    );
                }
            });
        });
    }

    for n in BENCH_SIZES {
        inner(c, *n);
    }
}

fn delete_by_hashed_non_unique_key_benchmark(c: &mut Criterion) {
    fn inner(c: &mut Criterion, n: u32) {
        let mut map = black_box(MultiIndexTestElementWithOnlyIndexedFieldsMap::default());
        for i in 0..n {
            map.insert(black_box(TestElementWithOnlyIndexedFields {
                field_hashed_unique: TestNonPrimitiveType(i),
                field_hashed_non_unique: TestNonPrimitiveType(42 + i % 5),
                field_ordered_unique: i,
                field_ordered_non_unique: i / 5,
            }));
        }
        c.bench_function(&format!("delete_hashed_non_unique_key_bench_{n}"), |b| {
            b.iter(|| {
                let mut map_clone = black_box(map.clone());
                for i in 0..10 {
                    black_box(
                        map_clone
                            .by_mut::<ByFieldHashedNonUnique>()
                            .remove_all(black_box(&TestNonPrimitiveType(42 + i % 5))),
                    );
                }
            });
        });
    }

    for n in BENCH_SIZES {
        inner(c, *n);
    }
}

fn delete_by_ordered_unique_key_benchmark(c: &mut Criterion) {
    fn inner(c: &mut Criterion, n: u32) {
        let mut map = black_box(MultiIndexTestElementWithOnlyIndexedFieldsMap::default());
        for i in 0..n {
            map.insert(black_box(TestElementWithOnlyIndexedFields {
                field_hashed_unique: TestNonPrimitiveType(i),
                field_hashed_non_unique: TestNonPrimitiveType(42 + i % 5),
                field_ordered_unique: i,
                field_ordered_non_unique: i / 5,
            }));
        }
        c.bench_function(&format!("delete_ordered_unique_key_bench_{n}"), |b| {
            b.iter(|| {
                let mut map_clone = black_box(map.clone());
                for i in 0..n {
                    black_box(
                        map_clone
                            .by_mut::<ByFieldOrderedUnique>()
                            .remove(black_box(&i)),
                    );
                }
            });
        });
    }

    for n in BENCH_SIZES {
        inner(c, *n);
    }
}

fn delete_by_ordered_non_unique_key_benchmark(c: &mut Criterion) {
    fn inner(c: &mut Criterion, n: u32) {
        let mut map = black_box(MultiIndexTestElementWithOnlyIndexedFieldsMap::default());
        for i in 0..n {
            map.insert(black_box(TestElementWithOnlyIndexedFields {
                field_hashed_unique: TestNonPrimitiveType(i),
                field_hashed_non_unique: TestNonPrimitiveType(42 + i % 5),
                field_ordered_unique: i,
                field_ordered_non_unique: i / 5,
            }));
        }
        c.bench_function(&format!("delete_ordered_non_unique_key_bench_{n}"), |b| {
            b.iter(|| {
                let mut map_clone = black_box(map.clone());
                for i in 0..n {
                    black_box(
                        map_clone
                            .by_mut::<ByFieldOrderedNonUnique>()
                            .remove_all(black_box(&(i / 5))),
                    );
                }
            });
        });
    }

    for n in BENCH_SIZES {
        inner(c, *n);
    }
}

fn modify_hashed_unique_key_by_hashed_unique_key_benchmark(c: &mut Criterion) {
    fn inner(c: &mut Criterion, n: u32) {
        let mut map = black_box(MultiIndexTestElementWithOnlyIndexedFieldsMap::default());
        for i in 0..n {
            map.insert(black_box(TestElementWithOnlyIndexedFields {
                field_hashed_unique: TestNonPrimitiveType(i),
                field_hashed_non_unique: TestNonPrimitiveType(42 + i % 5),
                field_ordered_unique: i,
                field_ordered_non_unique: i / 5,
            }));
        }
        c.bench_function(
            &format!("modify_hashed_unique_key_by_hashed_unique_key_bench_{n}"),
            |b| {
                b.iter(|| {
                    let mut map_clone = black_box(map.clone());
                    for i in 0..n {
                        black_box(
                            map_clone
                                .by_mut::<ByFieldHashedUnique>()
                                .modify(
                                    black_box(&TestNonPrimitiveType(i)),
                                    |e: &mut TestElementWithOnlyIndexedFields| {
                                        e.field_hashed_unique = black_box(TestNonPrimitiveType(
                                            e.field_hashed_unique.0 + n,
                                        ));
                                    },
                                )
                                .unwrap(),
                        );
                    }
                });
            },
        );
    }

    for n in BENCH_SIZES {
        inner(c, *n);
    }
}

fn modify_hashed_non_unique_key_by_hashed_unique_key_benchmark(c: &mut Criterion) {
    fn inner(c: &mut Criterion, n: u32) {
        let mut map = black_box(MultiIndexTestElementWithOnlyIndexedFieldsMap::default());
        for i in 0..n {
            map.insert(black_box(TestElementWithOnlyIndexedFields {
                field_hashed_unique: TestNonPrimitiveType(i),
                field_hashed_non_unique: TestNonPrimitiveType(42 + i % 5),
                field_ordered_unique: i,
                field_ordered_non_unique: i / 5,
            }));
        }
        c.bench_function(
            &format!("modify_hashed_non_unique_key_by_hashed_unique_key_bench_{n}"),
            |b| {
                b.iter(|| {
                    let mut map_clone = black_box(map.clone());
                    for i in 0..n {
                        black_box(
                            map_clone
                                .by_mut::<ByFieldHashedUnique>()
                                .modify(
                                    black_box(&TestNonPrimitiveType(i)),
                                    |e: &mut TestElementWithOnlyIndexedFields| {
                                        e.field_hashed_non_unique = black_box(
                                            TestNonPrimitiveType(e.field_hashed_non_unique.0 + 1),
                                        );
                                    },
                                )
                                .unwrap(),
                        );
                    }
                });
            },
        );
    }

    for n in BENCH_SIZES {
        inner(c, *n);
    }
}

fn modify_ordered_unique_key_by_hashed_unique_key_benchmark(c: &mut Criterion) {
    fn inner(c: &mut Criterion, n: u32) {
        let mut map = black_box(MultiIndexTestElementWithOnlyIndexedFieldsMap::default());
        for i in 0..n {
            map.insert(black_box(TestElementWithOnlyIndexedFields {
                field_hashed_unique: TestNonPrimitiveType(i),
                field_hashed_non_unique: TestNonPrimitiveType(42 + i % 5),
                field_ordered_unique: i,
                field_ordered_non_unique: i / 5,
            }));
        }
        c.bench_function(
            &format!("modify_ordered_unique_key_by_hashed_unique_key_bench_{n}"),
            |b| {
                b.iter(|| {
                    let mut map_clone = black_box(map.clone());
                    for i in 0..n {
                        black_box(
                            map_clone
                                .by_mut::<ByFieldHashedUnique>()
                                .modify(
                                    black_box(&TestNonPrimitiveType(i)),
                                    |e: &mut TestElementWithOnlyIndexedFields| {
                                        e.field_ordered_unique += black_box(n);
                                    },
                                )
                                .unwrap(),
                        );
                    }
                });
            },
        );
    }

    for n in BENCH_SIZES {
        inner(c, *n);
    }
}

fn modify_ordered_non_unique_key_by_hashed_unique_key_benchmark(c: &mut Criterion) {
    fn inner(c: &mut Criterion, n: u32) {
        let mut map = black_box(MultiIndexTestElementWithOnlyIndexedFieldsMap::default());
        for i in 0..n {
            map.insert(black_box(TestElementWithOnlyIndexedFields {
                field_hashed_unique: TestNonPrimitiveType(i),
                field_hashed_non_unique: TestNonPrimitiveType(42 + i % 5),
                field_ordered_unique: i,
                field_ordered_non_unique: i / 5,
            }));
        }
        c.bench_function(
            &format!("modify_ordered_non_unique_key_by_hashed_unique_key_bench_{n}"),
            |b| {
                b.iter(|| {
                    let mut map_clone = black_box(map.clone());
                    for i in 0..n {
                        black_box(
                            map_clone
                                .by_mut::<ByFieldHashedUnique>()
                                .modify(
                                    black_box(&TestNonPrimitiveType(i)),
                                    black_box(|e: &mut TestElementWithOnlyIndexedFields| {
                                        e.field_ordered_non_unique += black_box(1);
                                    }),
                                )
                                .unwrap(),
                        );
                    }
                });
            },
        );
    }

    for n in BENCH_SIZES {
        inner(c, *n);
    }
}

fn modify_hashed_unique_key_by_ordered_unique_key_benchmark(c: &mut Criterion) {
    fn inner(c: &mut Criterion, n: u32) {
        let mut map = black_box(MultiIndexTestElementWithOnlyIndexedFieldsMap::default());
        for i in 0..n {
            map.insert(black_box(TestElementWithOnlyIndexedFields {
                field_hashed_unique: TestNonPrimitiveType(i),
                field_hashed_non_unique: TestNonPrimitiveType(42 + i % 5),
                field_ordered_unique: i,
                field_ordered_non_unique: i / 5,
            }));
        }
        c.bench_function(
            &format!("modify_hashed_unique_key_by_ordered_unique_key_bench_{n}"),
            |b| {
                b.iter(|| {
                    let mut map_clone = black_box(map.clone());
                    for i in 0..n {
                        black_box(
                            map_clone
                                .by_mut::<ByFieldOrderedUnique>()
                                .modify(
                                    black_box(&i),
                                    |e: &mut TestElementWithOnlyIndexedFields| {
                                        e.field_hashed_unique = black_box(TestNonPrimitiveType(
                                            e.field_hashed_unique.0 + n,
                                        ));
                                    },
                                )
                                .unwrap(),
                        );
                    }
                });
            },
        );
    }

    for n in BENCH_SIZES {
        inner(c, *n);
    }
}

fn modify_hashed_non_unique_key_by_ordered_unique_key_benchmark(c: &mut Criterion) {
    fn inner(c: &mut Criterion, n: u32) {
        let mut map = black_box(MultiIndexTestElementWithOnlyIndexedFieldsMap::default());
        for i in 0..n {
            map.insert(black_box(TestElementWithOnlyIndexedFields {
                field_hashed_unique: TestNonPrimitiveType(i),
                field_hashed_non_unique: TestNonPrimitiveType(42 + i % 5),
                field_ordered_unique: i,
                field_ordered_non_unique: i / 5,
            }));
        }
        c.bench_function(
            &format!("modify_hashed_non_unique_key_by_ordered_unique_key_bench_{n}"),
            |b| {
                b.iter(|| {
                    let mut map_clone = black_box(map.clone());
                    for i in 0..n {
                        black_box(
                            map_clone
                                .by_mut::<ByFieldOrderedUnique>()
                                .modify(
                                    black_box(&i),
                                    |e: &mut TestElementWithOnlyIndexedFields| {
                                        e.field_hashed_non_unique = black_box(
                                            TestNonPrimitiveType(e.field_hashed_non_unique.0 + 1),
                                        );
                                    },
                                )
                                .unwrap(),
                        );
                    }
                });
            },
        );
    }

    for n in BENCH_SIZES {
        inner(c, *n);
    }
}

fn modify_ordered_unique_key_by_ordered_unique_key_benchmark(c: &mut Criterion) {
    fn inner(c: &mut Criterion, n: u32) {
        let mut map = black_box(MultiIndexTestElementWithOnlyIndexedFieldsMap::default());
        for i in 0..n {
            map.insert(black_box(TestElementWithOnlyIndexedFields {
                field_hashed_unique: TestNonPrimitiveType(i),
                field_hashed_non_unique: TestNonPrimitiveType(42 + i % 5),
                field_ordered_unique: i,
                field_ordered_non_unique: i / 5,
            }));
        }
        c.bench_function(
            &format!("modify_ordered_unique_key_by_ordered_unique_key_bench_{n}"),
            |b| {
                b.iter(|| {
                    let mut map_clone = black_box(map.clone());
                    for i in 0..n {
                        black_box(
                            map_clone
                                .by_mut::<ByFieldOrderedUnique>()
                                .modify(
                                    black_box(&i),
                                    |e: &mut TestElementWithOnlyIndexedFields| {
                                        e.field_ordered_unique += black_box(n);
                                    },
                                )
                                .unwrap(),
                        );
                    }
                });
            },
        );
    }

    for n in BENCH_SIZES {
        inner(c, *n);
    }
}

fn modify_ordered_non_unique_key_by_ordered_unique_key_benchmark(c: &mut Criterion) {
    fn inner(c: &mut Criterion, n: u32) {
        let mut map = black_box(MultiIndexTestElementWithOnlyIndexedFieldsMap::default());
        for i in 0..n {
            map.insert(black_box(TestElementWithOnlyIndexedFields {
                field_hashed_unique: TestNonPrimitiveType(i),
                field_hashed_non_unique: TestNonPrimitiveType(42 + i % 5),
                field_ordered_unique: i,
                field_ordered_non_unique: i / 5,
            }));
        }
        c.bench_function(
            &format!("modify_ordered_non_unique_key_by_ordered_unique_key_bench_{n}"),
            |b| {
                b.iter(|| {
                    let mut map_clone = black_box(map.clone());
                    for i in 0..n {
                        black_box(
                            map_clone
                                .by_mut::<ByFieldOrderedUnique>()
                                .modify(
                                    black_box(&i),
                                    |e: &mut TestElementWithOnlyIndexedFields| {
                                        e.field_ordered_non_unique += black_box(1);
                                    },
                                )
                                .unwrap(),
                        );
                    }
                });
            },
        );
    }

    for n in BENCH_SIZES {
        inner(c, *n);
    }
}

fn hashed_unique_key_iter_benchmark(c: &mut Criterion) {
    fn inner(c: &mut Criterion, n: u32) {
        let mut map = black_box(MultiIndexTestElementWithOnlyIndexedFieldsMap::default());
        for i in 0..n {
            map.insert(black_box(TestElementWithOnlyIndexedFields {
                field_hashed_unique: TestNonPrimitiveType(i),
                field_hashed_non_unique: TestNonPrimitiveType(42),
                field_ordered_unique: i,
                field_ordered_non_unique: i / 5,
            }));
            map.clear();
        }
        c.bench_function(&format!("hashed_unique_key_iter_bench_{n}"), |b| {
            b.iter(|| {
                let mut a = black_box(map.by::<ByFieldHashedUnique>().iter());
                for _ in 0..n {
                    black_box(a.next());
                }
            });
        });
    }

    for n in BENCH_SIZES {
        inner(c, *n);
    }
}

fn hashed_non_unique_key_iter_benchmark(c: &mut Criterion) {
    fn inner(c: &mut Criterion, n: u32) {
        let mut map = black_box(MultiIndexTestElementWithOnlyIndexedFieldsMap::default());
        for i in 0..n {
            map.insert(black_box(TestElementWithOnlyIndexedFields {
                field_hashed_unique: TestNonPrimitiveType(i),
                field_hashed_non_unique: TestNonPrimitiveType(42),
                field_ordered_unique: i,
                field_ordered_non_unique: i / 5,
            }));
            map.clear();
        }
        c.bench_function(&format!("hashed_non_unique_key_iter_bench_{n}"), |b| {
            b.iter(|| {
                let mut a = black_box(map.by::<ByFieldHashedNonUnique>().iter());
                for _ in 0..n {
                    black_box(a.next());
                }
            });
        });
    }

    for n in BENCH_SIZES {
        inner(c, *n);
    }
}

fn ordered_unique_key_iter_benchmark(c: &mut Criterion) {
    fn inner(c: &mut Criterion, n: u32) {
        let mut map = black_box(MultiIndexTestElementWithOnlyIndexedFieldsMap::default());
        for i in 0..n {
            map.insert(black_box(TestElementWithOnlyIndexedFields {
                field_hashed_unique: TestNonPrimitiveType(i),
                field_hashed_non_unique: TestNonPrimitiveType(42),
                field_ordered_unique: i,
                field_ordered_non_unique: i / 5,
            }));
            map.clear();
        }
        c.bench_function(&format!("ordered_unique_key_iter_bench_{n}"), |b| {
            b.iter(|| {
                let mut a = black_box(map.by::<ByFieldOrderedUnique>().iter());
                for _ in 0..n {
                    black_box(a.next());
                }
            });
        });
    }

    for n in BENCH_SIZES {
        inner(c, *n);
    }
}

fn ordered_non_unique_key_iter_benchmark(c: &mut Criterion) {
    fn inner(c: &mut Criterion, n: u32) {
        let mut map = black_box(MultiIndexTestElementWithOnlyIndexedFieldsMap::default());
        for i in 0..n {
            map.insert(black_box(TestElementWithOnlyIndexedFields {
                field_hashed_unique: TestNonPrimitiveType(i),
                field_hashed_non_unique: TestNonPrimitiveType(42),
                field_ordered_unique: i,
                field_ordered_non_unique: i / 5,
            }));
            map.clear();
        }
        c.bench_function(&format!("ordered_non_unique_key_iter_bench_{n}"), |b| {
            b.iter(|| {
                let mut a = black_box(map.by::<ByFieldOrderedNonUnique>().iter());
                for _ in 0..n {
                    black_box(a.next());
                }
            });
        });
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
    hashed_unique_key_iter_benchmark,
    hashed_non_unique_key_iter_benchmark,
    ordered_unique_key_iter_benchmark,
    ordered_non_unique_key_iter_benchmark,
);

criterion_main!(benches);
