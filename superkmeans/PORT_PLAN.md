# Hierarchical SuperKMeans Rust Port Plan

Source: `cwida/SuperKMeans` `main` at `bb63fb560283de2759f74ea0f00386c42ece9b64`.

Scope: exact Rust-facing port of upstream `HierarchicalSuperKMeans<Quantization::f32, DistanceFunction::l2>`, matching the instantiation used by upstream Python bindings, examples, benchmarks, and tests. The hierarchical Rust API is backed directly by the vendored upstream C++ headers through `cpp/superkmeans_bridge.cpp`; the earlier native Rust kernel port has been removed so there is only one implementation path.

## Exactness Plan

1. Vendor the upstream headers under `third_party/superkmeans/include`.
2. Compile a small C++17 bridge in `build.rs` with OpenMP, BLAS/Accelerate on macOS, Eigen, optional FFTW when present, and upstream release-style optimization flags.
3. Instantiate the same upstream type used upstream: `skmeans::HierarchicalSuperKMeans<skmeans::Quantization::f32, skmeans::DistanceFunction::l2>`.
4. Expose a C ABI for the hierarchical constructor, training, assignment, training-point assignment, config sync, iteration stats, distance pointer access, mesocluster/sample helpers, and cluster balance stats.
5. Keep the public Rust config/stat structs as Rust-native mirrors, but populate effective config and stats from the C++ object.
6. Validate Rust slice lengths before crossing FFI to avoid undefined behavior; algorithmic behavior after validation is upstream C++ behavior.

## Hierarchical Call Graph Map

| Upstream function or state                        | Rust target                                    | Exactness path                                                 |
| ------------------------------------------------- | ---------------------------------------------- | -------------------------------------------------------------- |
| `HierarchicalSuperKMeansConfig()`                 | `HierarchicalSuperKMeansConfig::default`       | Rust mirror of upstream defaults                               |
| `HierarchicalSuperKMeans(...)`                    | `HierarchicalSuperKMeans::with_config/new`     | `skm_hier_new` constructs upstream C++ object                  |
| effective `hierarchical_config` after parent sync | `HierarchicalSuperKMeans::hierarchical_config` | copied back from C++ via `skm_hier_copy_config`                |
| `Train(data, n, queries, n_queries)`              | `train` / `train_with_queries`                 | direct C++ call via `skm_hier_train`                           |
| `Assign`                                          | `assign`                                       | direct C++ call via `skm_hier_assign`                          |
| `AssignTrainingPoints`                            | `assign_training_points`                       | direct C++ call via `skm_hier_assign_training_points`          |
| `GetNClusters`                                    | `get_n_clusters`                               | direct C++ call                                                |
| `IsTrained`                                       | `is_trained`                                   | direct C++ call                                                |
| `GetSamplingFraction`                             | `get_sampling_fraction`                        | direct C++ call                                                |
| `GetDistancesPointer`                             | `get_distances`                                | borrowed from C++ object                                       |
| `GetNMesoclusters`                                | `get_n_mesoclusters`                           | direct C++ static call                                         |
| `GetNVectorsToSample`                             | `get_n_vectors_to_sample`                      | direct C++ virtual call                                        |
| `GetClustersBalanceStats`                         | `get_clusters_balance_stats`                   | direct C++ static call                                         |
| `hierarchical_iteration_stats`                    | `hierarchical_iteration_stats`                 | copied from C++ object after training                          |
| inherited `iteration_stats`                       | `iteration_stats`                              | preserved as the upstream hierarchical empty base stats vector |

## Upstream Internal Functions Covered

Because training and assignment execute inside upstream C++, the following called functions are not reimplemented independently on the hierarchical path; they are the upstream functions:

| Upstream internal function                                  | Called by                                                 |
| ----------------------------------------------------------- | --------------------------------------------------------- |
| `SuperKMeans` constructor and parent config synchronization | C++ hierarchical constructor                              |
| `ADSamplingPruner` constructor                              | C++ hierarchical constructor                              |
| `GenerateCentroids`                                         | `Train` mesoclustering/fineclustering                     |
| `SampleAndRotateVectors`                                    | `Train`                                                   |
| `RotateOrCopy`                                              | `Train`, `AssignTrainingPoints`                           |
| `GetL2NormsRowMajor`                                        | `Train`, `Assign`                                         |
| `GetPartialL2NormsRowMajor`                                 | pruning iterations/refinement/assignment                  |
| `RunIteration<true>`                                        | GEMM-only mesoclustering, fineclustering, refinement      |
| `RunIteration<false>`                                       | pruning-backed mesoclustering, fineclustering, refinement |
| `FirstAssignAndUpdateCentroids`                             | first iteration path                                      |
| `AssignAndUpdateCentroids`                                  | non-first iteration path                                  |
| `UpdateCentroids` / `UpdateCentroid`                        | iteration centroid updates                                |
| `ConsolidateCentroids`                                      | iteration finalization                                    |
| hierarchical `SplitClusters` override                       | empty/small cluster handling                              |
| `ComputeCost` / `ComputeShift`                              | iteration stats                                           |
| `ShouldStopEarly`                                           | early termination                                         |
| `CentroidsToAuxiliaryHorizontal`                            | PDX auxiliary state                                       |
| `TunePartialD`                                              | pruning partial dimension tuning                          |
| `GetOutputCentroids`                                        | train return value                                        |
| `PostprocessCentroids`                                      | angular mode                                              |
| `CompactMesoclusterToBuffer`                                | fineclustering                                            |
| `ArrangeFineClusters`                                       | fineclustering allocation                                 |
| `GetTrueAssignmentsFromIndirectionBuffer`                   | fineclustering assignment remap                           |
| `SetupCentroids`                                            | refinement                                                |
| `BatchComputer::FindNearestNeighbor`                        | `Assign` and GEMM-only paths                              |
| `BatchComputer::FindNearestNeighborWithPruning`             | pruning assignment paths                                  |
| `PDXLayout` / `PDXearch` helpers                            | pruning paths                                             |

## Audit Result

The hierarchical Rust API now delegates the algorithmically relevant `f32`/L2 surface to the vendored upstream C++ implementation. No independent Rust hierarchical training, assignment, pruning, RNG, rotation, centroid splitting, or stats logic remains on that path.
