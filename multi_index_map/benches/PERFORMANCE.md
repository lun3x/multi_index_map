# MultiIndexMap2 Performance Report

Measured on June 10, 2026 with `cargo bench --bench performance`. Criterion
times are machine-specific representative medians. The suite uses equivalent
10,000-element fixtures for:

- `old`: the existing `MultiIndexMap`
- `selector`: `MultiIndexMap2` through selector views
- `compatibility`: legacy-syntax `MultiIndexMap2` through deprecated wrappers

Population and cloning are excluded from timed mutation setup. Selector
equal-range traversal is lazy; compatibility and old `get_by_*` results include
their `Vec` allocation.

## Final 10,000-Element Results

| Workload | Old | Selector | Compatibility |
| --- | ---: | ---: | ---: |
| Clone | 272 us | 692 us | 692 us |
| Growing insert | 1.27 ms | 1.87 ms | 1.88 ms |
| Clear | 155 us | 15.6 us | 14.0 us |
| Remove all through hashed unique | 805 us | 299 us | 784 us |
| Modify without changing keys | 193 us | 205 us | 1.14 ms |
| Relocate an ordered key | 668 us | 984 us | 1.96 ms |
| Iterate hashed unique | 9.49 us | 6.07 us | 6.05 us |
| Iterate ordered unique | 14.4 us | 59.1 us | 65.6 us |
| Hashed non-unique equal range / `get_by_*` | 1.16 us | 2.39 us | 1.65 us |

The compatibility facade can be materially slower when it must snapshot IDs or
allocate result vectors. Selector views are the engine-cost comparison.

## Accepted Changes

| Change | Main measured effect |
| --- | --- |
| Structural clone | Selector clone fell from about 35.7 ms to below 0.8 ms. |
| Bulk clear | Selector clear fell from about 918 us to about 15 us. |
| Compact links and direct hashed values iteration | `Option<NodeId>` became one word; `HashLink` and `OrderedLink` are 32 bytes. Hashed full iteration fell to about 6 us. |
| Incremental ordered extrema | Ordered removal improved by roughly 50 percent. |
| Safe hashed hot paths | Growing insertion fell from about 29 ms to about 1.8 ms; unchanged modification fell from about 44 ms to about 0.2 ms. |
| Replacement preflight | Successful replacement improved about 35 percent; repeated conflicting replacement improved about 89 percent. |

## Deferred Conditional Work

- The advanced tagged Boost hash layout is deferred. Lookup, removal, and full
  hashed iteration now beat the old map broadly; the remaining hash equal-range
  gap alone does not justify the representation complexity yet.
- Unsafe arena access is deferred. Sampling did not identify checked `Slab`
  access as a dominant named frame, so the evidence gate for unsafe code was
  not met.
- Ordered iteration remains the clearest residual engine target.
