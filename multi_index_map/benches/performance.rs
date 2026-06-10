use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use multi_index_map::{MultiIndexMap, MultiIndexMap2, MultiIndexSelector};
use std::hint::black_box;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TestKey(u32);

#[derive(MultiIndexSelector)]
#[multi_index(hashed_unique)]
pub struct ByHashedUnique;

#[derive(MultiIndexSelector)]
#[multi_index(hashed_non_unique)]
pub struct ByHashedNonUnique;

#[derive(MultiIndexSelector)]
#[multi_index(ordered_unique)]
pub struct ByOrderedUnique;

#[derive(MultiIndexSelector)]
#[multi_index(ordered_non_unique)]
pub struct ByOrderedNonUnique;

#[allow(clippy::struct_field_names)]
#[derive(Clone, MultiIndexMap)]
#[multi_index_derive(Clone)]
pub struct OldElement {
    #[multi_index(hashed_unique)]
    field_hashed_unique: TestKey,
    #[multi_index(hashed_non_unique)]
    field_hashed_non_unique: TestKey,
    #[multi_index(ordered_unique)]
    field_ordered_unique: u32,
    #[multi_index(ordered_non_unique)]
    field_ordered_non_unique: u32,
}

#[allow(clippy::struct_field_names)]
#[derive(Clone, MultiIndexMap2)]
#[multi_index_derive(Clone)]
pub struct SelectorElement {
    #[multi_index(by(ByHashedUnique))]
    field_hashed_unique: TestKey,
    #[multi_index(by(ByHashedNonUnique))]
    field_hashed_non_unique: TestKey,
    #[multi_index(by(ByOrderedUnique))]
    field_ordered_unique: u32,
    #[multi_index(by(ByOrderedNonUnique))]
    field_ordered_non_unique: u32,
}

#[allow(clippy::struct_field_names)]
#[derive(Clone, MultiIndexMap2)]
#[multi_index_derive(Clone)]
pub struct CompatibilityElement {
    #[multi_index(hashed_unique)]
    field_hashed_unique: TestKey,
    #[multi_index(hashed_non_unique)]
    field_hashed_non_unique: TestKey,
    #[multi_index(ordered_unique)]
    field_ordered_unique: u32,
    #[multi_index(ordered_non_unique)]
    field_ordered_non_unique: u32,
}

const BENCH_SIZES: &[u32] = &[100, 1_000, 10_000];

fn old_element(i: u32) -> OldElement {
    OldElement {
        field_hashed_unique: TestKey(i),
        field_hashed_non_unique: TestKey(42 + i % 20),
        field_ordered_unique: i,
        field_ordered_non_unique: i / 5,
    }
}

fn selector_element(i: u32) -> SelectorElement {
    SelectorElement {
        field_hashed_unique: TestKey(i),
        field_hashed_non_unique: TestKey(42 + i % 20),
        field_ordered_unique: i,
        field_ordered_non_unique: i / 5,
    }
}

fn compatibility_element(i: u32) -> CompatibilityElement {
    CompatibilityElement {
        field_hashed_unique: TestKey(i),
        field_hashed_non_unique: TestKey(42 + i % 20),
        field_ordered_unique: i,
        field_ordered_non_unique: i / 5,
    }
}

fn old_map(n: u32) -> MultiIndexOldElementMap {
    let mut map = MultiIndexOldElementMap::default();
    for i in 0..n {
        map.insert(old_element(i));
    }
    map
}

fn selector_map(n: u32) -> MultiIndexSelectorElementMap {
    let mut map = MultiIndexSelectorElementMap::default();
    for i in 0..n {
        map.insert(selector_element(i));
    }
    map
}

fn compatibility_map(n: u32) -> MultiIndexCompatibilityElementMap {
    let mut map = MultiIndexCompatibilityElementMap::default();
    for i in 0..n {
        map.insert(compatibility_element(i));
    }
    map
}

fn bench_clones(c: &mut Criterion) {
    for &n in BENCH_SIZES {
        let old = old_map(n);
        let selector = selector_map(n);
        let compatibility = compatibility_map(n);
        c.bench_function(&format!("old/clone/{n}"), |b| {
            b.iter(|| black_box(old.clone()))
        });
        c.bench_function(&format!("selector/clone/{n}"), |b| {
            b.iter(|| black_box(selector.clone()))
        });
        c.bench_function(&format!("compatibility/clone/{n}"), |b| {
            b.iter(|| black_box(compatibility.clone()))
        });
    }
}

fn bench_insert_and_clear(c: &mut Criterion) {
    for &n in BENCH_SIZES {
        c.bench_function(&format!("old/insert_growing/{n}"), |b| {
            b.iter_batched(
                MultiIndexOldElementMap::default,
                |mut map| {
                    for i in 0..n {
                        black_box(map.insert(black_box(old_element(i))));
                    }
                },
                BatchSize::LargeInput,
            )
        });
        c.bench_function(&format!("selector/insert_growing/{n}"), |b| {
            b.iter_batched(
                MultiIndexSelectorElementMap::default,
                |mut map| {
                    for i in 0..n {
                        black_box(map.insert(black_box(selector_element(i))));
                    }
                },
                BatchSize::LargeInput,
            )
        });
        c.bench_function(&format!("compatibility/insert_growing/{n}"), |b| {
            b.iter_batched(
                MultiIndexCompatibilityElementMap::default,
                |mut map| {
                    for i in 0..n {
                        black_box(map.insert(black_box(compatibility_element(i))));
                    }
                },
                BatchSize::LargeInput,
            )
        });

        let old = old_map(n);
        let selector = selector_map(n);
        let compatibility = compatibility_map(n);
        c.bench_function(&format!("old/clear/{n}"), |b| {
            b.iter_batched(|| old.clone(), |mut map| map.clear(), BatchSize::LargeInput)
        });
        c.bench_function(&format!("selector/clear/{n}"), |b| {
            b.iter_batched(
                || selector.clone(),
                |mut map| map.clear(),
                BatchSize::LargeInput,
            )
        });
        c.bench_function(&format!("compatibility/clear/{n}"), |b| {
            b.iter_batched(
                || compatibility.clone(),
                |mut map| map.clear(),
                BatchSize::LargeInput,
            )
        });
    }
}

#[allow(deprecated)]
fn bench_lookup_and_ranges(c: &mut Criterion) {
    for &n in BENCH_SIZES {
        let old = old_map(n);
        let selector = selector_map(n);
        let compatibility = compatibility_map(n);
        let hit = TestKey(n / 2);
        let miss = TestKey(n + 1);
        let group = TestKey(42 + (n / 2) % 20);

        c.bench_function(&format!("old/lookup_hashed_unique_hit/{n}"), |b| {
            b.iter(|| black_box(old.get_by_field_hashed_unique(black_box(&hit))))
        });
        c.bench_function(&format!("selector/lookup_hashed_unique_hit/{n}"), |b| {
            b.iter(|| black_box(selector.by::<ByHashedUnique>().get(black_box(&hit))))
        });
        c.bench_function(
            &format!("compatibility/lookup_hashed_unique_hit/{n}"),
            |b| b.iter(|| black_box(compatibility.get_by_field_hashed_unique(black_box(&hit)))),
        );

        c.bench_function(&format!("old/lookup_hashed_unique_miss/{n}"), |b| {
            b.iter(|| black_box(old.get_by_field_hashed_unique(black_box(&miss))))
        });
        c.bench_function(&format!("selector/lookup_hashed_unique_miss/{n}"), |b| {
            b.iter(|| black_box(selector.by::<ByHashedUnique>().get(black_box(&miss))))
        });
        c.bench_function(
            &format!("compatibility/lookup_hashed_unique_miss/{n}"),
            |b| b.iter(|| black_box(compatibility.get_by_field_hashed_unique(black_box(&miss)))),
        );

        c.bench_function(&format!("old/lookup_ordered_unique_hit/{n}"), |b| {
            b.iter(|| black_box(old.get_by_field_ordered_unique(black_box(&(n / 2)))))
        });
        c.bench_function(&format!("selector/lookup_ordered_unique_hit/{n}"), |b| {
            b.iter(|| black_box(selector.by::<ByOrderedUnique>().get(black_box(&(n / 2)))))
        });
        c.bench_function(
            &format!("compatibility/lookup_ordered_unique_hit/{n}"),
            |b| {
                b.iter(|| black_box(compatibility.get_by_field_ordered_unique(black_box(&(n / 2)))))
            },
        );

        c.bench_function(&format!("old/compat_get_hashed_non_unique/{n}"), |b| {
            b.iter(|| black_box(old.get_by_field_hashed_non_unique(black_box(&group))))
        });
        c.bench_function(
            &format!("selector/equal_range_hashed_non_unique/{n}"),
            |b| {
                b.iter(|| {
                    black_box(
                        selector
                            .by::<ByHashedNonUnique>()
                            .equal_range(black_box(&group))
                            .count(),
                    )
                })
            },
        );
        c.bench_function(
            &format!("compatibility/compat_get_hashed_non_unique/{n}"),
            |b| {
                b.iter(|| {
                    black_box(compatibility.get_by_field_hashed_non_unique(black_box(&group)))
                })
            },
        );

        c.bench_function(&format!("old/compat_get_ordered_non_unique/{n}"), |b| {
            b.iter(|| black_box(old.get_by_field_ordered_non_unique(black_box(&(n / 10)))))
        });
        c.bench_function(
            &format!("selector/equal_range_ordered_non_unique/{n}"),
            |b| {
                b.iter(|| {
                    black_box(
                        selector
                            .by::<ByOrderedNonUnique>()
                            .equal_range(black_box(&(n / 10)))
                            .count(),
                    )
                })
            },
        );
        c.bench_function(
            &format!("compatibility/compat_get_ordered_non_unique/{n}"),
            |b| {
                b.iter(|| {
                    black_box(compatibility.get_by_field_ordered_non_unique(black_box(&(n / 10))))
                })
            },
        );
    }
}

#[allow(deprecated)]
fn bench_remove(c: &mut Criterion) {
    for &n in BENCH_SIZES {
        let old = old_map(n);
        let selector = selector_map(n);
        let compatibility = compatibility_map(n);
        c.bench_function(&format!("old/remove_hashed_unique_all/{n}"), |b| {
            b.iter_batched(
                || old.clone(),
                |mut map| {
                    for i in 0..n {
                        black_box(map.remove_by_field_hashed_unique(black_box(&TestKey(i))));
                    }
                },
                BatchSize::LargeInput,
            )
        });
        c.bench_function(&format!("selector/remove_hashed_unique_all/{n}"), |b| {
            b.iter_batched(
                || selector.clone(),
                |mut map| {
                    for i in 0..n {
                        black_box(
                            map.by_mut::<ByHashedUnique>()
                                .remove(black_box(&TestKey(i))),
                        );
                    }
                },
                BatchSize::LargeInput,
            )
        });
        c.bench_function(
            &format!("compatibility/remove_hashed_unique_all/{n}"),
            |b| {
                b.iter_batched(
                    || compatibility.clone(),
                    |mut map| {
                        for i in 0..n {
                            black_box(map.remove_by_field_hashed_unique(black_box(&TestKey(i))));
                        }
                    },
                    BatchSize::LargeInput,
                )
            },
        );

        c.bench_function(&format!("old/remove_ordered_unique_all/{n}"), |b| {
            b.iter_batched(
                || old.clone(),
                |mut map| {
                    for i in 0..n {
                        black_box(map.remove_by_field_ordered_unique(black_box(&i)));
                    }
                },
                BatchSize::LargeInput,
            )
        });
        c.bench_function(&format!("selector/remove_ordered_unique_all/{n}"), |b| {
            b.iter_batched(
                || selector.clone(),
                |mut map| {
                    for i in 0..n {
                        black_box(map.by_mut::<ByOrderedUnique>().remove(black_box(&i)));
                    }
                },
                BatchSize::LargeInput,
            )
        });
        c.bench_function(
            &format!("compatibility/remove_ordered_unique_all/{n}"),
            |b| {
                b.iter_batched(
                    || compatibility.clone(),
                    |mut map| {
                        for i in 0..n {
                            black_box(map.remove_by_field_ordered_unique(black_box(&i)));
                        }
                    },
                    BatchSize::LargeInput,
                )
            },
        );

        c.bench_function(&format!("old/remove_hashed_non_unique_all/{n}"), |b| {
            b.iter_batched(
                || old.clone(),
                |mut map| {
                    for key in 42..62 {
                        black_box(map.remove_by_field_hashed_non_unique(black_box(&TestKey(key))));
                    }
                },
                BatchSize::LargeInput,
            )
        });
        c.bench_function(&format!("selector/remove_hashed_non_unique_all/{n}"), |b| {
            b.iter_batched(
                || selector.clone(),
                |mut map| {
                    for key in 42..62 {
                        black_box(
                            map.by_mut::<ByHashedNonUnique>()
                                .remove_all(black_box(&TestKey(key))),
                        );
                    }
                },
                BatchSize::LargeInput,
            )
        });
        c.bench_function(
            &format!("compatibility/remove_hashed_non_unique_all/{n}"),
            |b| {
                b.iter_batched(
                    || compatibility.clone(),
                    |mut map| {
                        for key in 42..62 {
                            black_box(
                                map.remove_by_field_hashed_non_unique(black_box(&TestKey(key))),
                            );
                        }
                    },
                    BatchSize::LargeInput,
                )
            },
        );
    }
}

#[allow(deprecated)]
fn bench_modify(c: &mut Criterion) {
    for &n in BENCH_SIZES {
        let old = old_map(n);
        let selector = selector_map(n);
        let compatibility = compatibility_map(n);

        c.bench_function(&format!("old/modify_unchanged/{n}"), |b| {
            b.iter_batched(
                || old.clone(),
                |mut map| {
                    for i in 0..n {
                        black_box(map.modify_by_field_hashed_unique(&TestKey(i), |value| {
                            black_box(&mut value.field_hashed_unique);
                        }));
                    }
                },
                BatchSize::LargeInput,
            )
        });
        c.bench_function(&format!("selector/modify_unchanged/{n}"), |b| {
            b.iter_batched(
                || selector.clone(),
                |mut map| {
                    for i in 0..n {
                        let _ = black_box(map.by_mut::<ByHashedUnique>().modify(
                            &TestKey(i),
                            |value| {
                                black_box(&mut value.field_hashed_unique);
                            },
                        ));
                    }
                },
                BatchSize::LargeInput,
            )
        });
        c.bench_function(&format!("compatibility/modify_unchanged/{n}"), |b| {
            b.iter_batched(
                || compatibility.clone(),
                |mut map| {
                    for i in 0..n {
                        black_box(map.modify_by_field_hashed_unique(&TestKey(i), |value| {
                            black_box(&mut value.field_hashed_unique);
                        }));
                    }
                },
                BatchSize::LargeInput,
            )
        });

        c.bench_function(&format!("old/modify_relocate_ordered/{n}"), |b| {
            b.iter_batched(
                || old.clone(),
                |mut map| {
                    for i in 0..n {
                        black_box(map.modify_by_field_hashed_unique(&TestKey(i), |value| {
                            value.field_ordered_unique += n;
                        }));
                    }
                },
                BatchSize::LargeInput,
            )
        });
        c.bench_function(&format!("selector/modify_relocate_ordered/{n}"), |b| {
            b.iter_batched(
                || selector.clone(),
                |mut map| {
                    for i in 0..n {
                        let _ = black_box(map.by_mut::<ByHashedUnique>().modify(
                            &TestKey(i),
                            |value| {
                                value.field_ordered_unique += n;
                            },
                        ));
                    }
                },
                BatchSize::LargeInput,
            )
        });
        c.bench_function(&format!("compatibility/modify_relocate_ordered/{n}"), |b| {
            b.iter_batched(
                || compatibility.clone(),
                |mut map| {
                    for i in 0..n {
                        black_box(map.modify_by_field_hashed_unique(&TestKey(i), |value| {
                            value.field_ordered_unique += n;
                        }));
                    }
                },
                BatchSize::LargeInput,
            )
        });

        c.bench_function(&format!("selector/modify_conflict/{n}"), |b| {
            b.iter_batched(
                || selector.clone(),
                |mut map| {
                    let _ = black_box(map.by_mut::<ByHashedUnique>().modify(
                        &TestKey(n - 1),
                        |value| {
                            value.field_hashed_unique = TestKey(0);
                        },
                    ));
                },
                BatchSize::LargeInput,
            )
        });
    }
}

#[allow(deprecated)]
fn bench_iteration(c: &mut Criterion) {
    for &n in BENCH_SIZES {
        let old = old_map(n);
        let selector = selector_map(n);
        let compatibility = compatibility_map(n);

        c.bench_function(&format!("old/iter_hashed_unique/{n}"), |b| {
            b.iter(|| {
                for value in old.iter_by_field_hashed_unique() {
                    black_box(value);
                }
            })
        });
        c.bench_function(&format!("selector/iter_hashed_unique/{n}"), |b| {
            b.iter(|| {
                for value in selector.by::<ByHashedUnique>().iter() {
                    black_box(value);
                }
            })
        });
        c.bench_function(&format!("compatibility/iter_hashed_unique/{n}"), |b| {
            b.iter(|| {
                for value in compatibility.iter_by_field_hashed_unique() {
                    black_box(value);
                }
            })
        });

        c.bench_function(&format!("old/iter_hashed_non_unique/{n}"), |b| {
            b.iter(|| {
                for value in old.iter_by_field_hashed_non_unique() {
                    black_box(value);
                }
            })
        });
        c.bench_function(&format!("selector/iter_hashed_non_unique/{n}"), |b| {
            b.iter(|| {
                for value in selector.by::<ByHashedNonUnique>().iter() {
                    black_box(value);
                }
            })
        });
        c.bench_function(&format!("compatibility/iter_hashed_non_unique/{n}"), |b| {
            b.iter(|| {
                for value in compatibility.iter_by_field_hashed_non_unique() {
                    black_box(value);
                }
            })
        });

        c.bench_function(&format!("old/iter_ordered_unique/{n}"), |b| {
            b.iter(|| {
                for value in old.iter_by_field_ordered_unique() {
                    black_box(value);
                }
            })
        });
        c.bench_function(&format!("selector/iter_ordered_unique/{n}"), |b| {
            b.iter(|| {
                for value in selector.by::<ByOrderedUnique>().iter() {
                    black_box(value);
                }
            })
        });
        c.bench_function(&format!("compatibility/iter_ordered_unique/{n}"), |b| {
            b.iter(|| {
                for value in compatibility.iter_by_field_ordered_unique() {
                    black_box(value);
                }
            })
        });
    }
}

criterion_group!(
    benches,
    bench_clones,
    bench_insert_and_clear,
    bench_lookup_and_ranges,
    bench_remove,
    bench_modify,
    bench_iteration,
);
criterion_main!(benches);
