## Graph Lake MVP Plan (Iceberg + DataFusion + `iceberg-datafusion`, CSR traversal, multi-graph, external-table linking via `ext_id`)

### Mission

Build a **Graph Lake** engine where:

* each **graph** is a **first-class Iceberg dataset** (tables-per-graph),
* **SQL** works out of the box via **DataFusion**,
* **GQL/MATCH** is compiled from your AST into DataFusion plans,
* graph traversal is fast via a **CSR sidecar index** tied to **Iceberg snapshots**,
* and you can **link non-graph Iceberg tables to nodes** using a stable **external identifier** (`ext_id`) so the system integrates cleanly with lakehouse tooling.

---

# 1) Design principles and best practices

### Iceberg best practices (ecosystem-friendly)

* **Tables are the source of truth.** Nodes/edges/tokens/manifests are all Iceberg tables; no bespoke metadata service.
* **Snapshot-consistent reads.** Graph queries pin to a specific Iceberg snapshot so CSR and table scans are consistent.
* **Namespace isolation.** Each graph is its own Iceberg namespace → easy discovery, governance, and avoids `graph_id` columns everywhere.

### DataFusion best practices (execution-friendly)

* **Use TableProviders for everything.** All table access goes through DataFusion’s catalog resolution (no manual Parquet file handling in compiler).
* **Traversal is a custom operator, not joins.** Expansion uses a custom `ExecutionPlan` so adjacency access is a slice read, not a self-join.
* **Demand-driven projection.** Only read node columns actually needed by WHERE/RETURN/link-joins.

---

# 2) Multi-graph layout (tables per graph)

## 2.1 Namespace convention

DataFusion catalog: `graphs`
Graph namespace: `<graph_name>`
Tables:

* `graphs.<g>.nodes`
* `graphs.<g>.edges`
* `graphs.<g>.label_tokens`
* `graphs.<g>.type_tokens`
* `graphs.<g>.graph_index_manifests`

This supports multiple graphs from day 1 without runtime hacks.

---

# 3) Node ↔ external table linking (new MVP feature)

## 3.1 Add a stable `ext_id` to nodes

In `graphs.<g>.nodes`, include:

* `node_id: long` (internal graph ID, for adjacency)
* `ext_id: string` (stable business key, used for lakehouse joins)
* `label_id: int`
* typed property columns (hot props)

**Why this is best for MVP**

* Works with any Iceberg table in the ecosystem (just a SQL join on `ext_id`)
* IDs remain stable across graph rebuilds and reindexing
* No need to force external systems to know `node_id`

## 3.2 How “linking” works in queries

A “linked” lakehouse table is just another Iceberg table, for example:

* `lake.crm.customers(customer_ext_id, …)`
* `lake.finance.transactions(customer_ext_id, amount, ts, …)`

Your compiler can emit standard DataFusion joins:

1. traverse graph to produce `a__node_id`
2. join `nodes` to get `a__ext_id` (and/or demanded props)
3. join external table on `external.customer_ext_id = a__ext_id`

No new graph operator is needed.

### Optional (future, still Iceberg-native): declarative link metadata

Later you can add:

* `graphs.<g>.node_links(label_id, ext_table, ext_key_col, node_ext_id_col, …)`
  to let the compiler auto-infer link joins. Not required for MVP.

---

# 4) Iceberg schemas (per graph)

## 4.1 `nodes`

Required:

* `node_id: long`
* `ext_id: string`  ✅ (for linking)
* `label_id: int`

Recommended (MVP hot properties only):

* `name: string`
* `age: int`
* `city: string`
* etc.

## 4.2 `edges`

Required:

* `src: long`
* `dst: long`
* `type_id: int`

Partitioning (MVP):

* `bucket(src, N)` with **N = 256** (tunable)

## 4.3 token tables (Iceberg-native metadata)

* `label_tokens(label_id int, label_name string)`
* `type_tokens(type_id int, type_name string)`

## 4.4 `graph_index_manifests` (Iceberg-native index registry)

* `snapshot_id: long` (edges snapshot id)
* `csr_root: string` (where CSR buckets live)
* `n_buckets: int`
* `created_at: timestamp`

---

# 5) Sidecar CSR index (read-optimized adjacency)

## 5.1 On-disk layout (local MVP)

`./index_root/<graph>/snapshot=<snapshot_id>/csr/bucket=<000..255>.arrow`

CSR is always **per graph** and **per edges snapshot**.

## 5.2 CSR file format (Arrow IPC)

One Arrow IPC file per bucket with a single RecordBatch:

* `srcs: int64[]` sorted unique sources present in bucket
* `offsets: uint32[]` length = `srcs.len + 1`
* `dsts: int64[]` flattened adjacency
* `types: int32[]` parallel to `dsts`

Build with edge order `(src, type_id, dst)` for type clustering.

---

# 6) DataFusion + Iceberg integration (using `iceberg-datafusion`)

## 6.1 Suggested crates

Core:

* `datafusion`
* `iceberg`
* `iceberg-datafusion`
* `arrow` / `parquet` (via DataFusion deps)
* `object_store` (local FS now; S3/GCS later)
* `tokio`

Nice-to-have:

* `lru` or `dashmap` (CSR bucket cache)
* `roaring` (future: frontier distinct / uniqueness)

## 6.2 Cargo feature guidance

Keep features aligned so Arrow/DataFusion versions match `iceberg-datafusion`.

* DataFusion: enable Parquet + object_store support as needed
* Iceberg: enable the catalog and object-store backend you use locally
* Ensure a single async runtime (`tokio`) across the stack

(Exact flags depend on your chosen versions; the key is **version alignment** between `datafusion`, `arrow`, `iceberg`, and `iceberg-datafusion`.)

## 6.3 Engine startup wiring (local MVP)

1. Create `SessionContext`
2. Build an Iceberg catalog pointing to local warehouse (filesystem object store)
3. Wrap it with `IcebergCatalogProvider` from `iceberg-datafusion`
4. Register it in DataFusion as catalog `"graphs"`

After that:

* SQL works: `SELECT * FROM graphs.mygraph.nodes`
* Your compiler resolves tables through the same catalog API

## 6.4 Snapshot-consistent reads for graph queries

For GQL execution, pin scans to a snapshot:

* Use static snapshot table providers (or equivalent session-level “as of snapshot” mechanism provided by the integration) so:

  * `nodes` scan and `edges` scan are consistent
  * CSR manifest row is selected for the same `edges` snapshot

---

# 7) CSR builder algorithm (local, streaming)

CLI:

* `build-csr --graph <g> --index-root ./index_root --buckets 256 [--snapshot <id>]`

Algorithm:

1. Resolve `graphs.<g>.edges` from DataFusion catalog
2. Choose snapshot id (default current, or provided)
3. Execute a single scan of edges and stream `(src, dst, type_id)`
4. Partition rows by `bucket = hash(src) % N`

   * MVP: in-memory partitioning is fine for small/medium graphs
   * if needed: spill per-bucket to temp files (future)
5. For each bucket:

   * sort by `(src, type_id, dst)`
   * build `srcs`, `offsets`, `dsts`, `types`
   * write Arrow IPC
6. Append a row into `graphs.<g>.graph_index_manifests` linking `snapshot_id → csr_root, n_buckets`

---

# 8) GQL AST → DataFusion plan compilation

## 8.1 Internal column naming (avoid dots)

Use stable internal names:

* node ids: `a__node_id`, `b__node_id`
* ext ids: `a__ext_id`
* properties: `b__name`, `b__city`

This avoids ambiguity across repeated joins.

## 8.2 Demand analysis pass (recommended)

Walk:

* RETURN expressions
* WHERE predicates
* property specs
* external table join expressions (linking)

Collect demanded columns:

* per variable: which node properties and whether `ext_id` is needed

## 8.3 Token resolution (Iceberg-native)

Per query/session:

* load `label_tokens` and `type_tokens` to maps:

  * `label_name -> label_id`
  * `type_name -> type_id`

Cache (MVP: per query is ok; per session is better).

## 8.4 Edge type filter strategy

* single label → pass `type_filter=Some(id)` into expand operator (fast path)
* disjunction (`A|B`) → expand all + `Filter type IN (idA, idB)`
* wildcard/none → no filter
* conjunction/negation → reject MVP

## 8.5 Lowering flow for one path chain

For query like:
`MATCH (a:Customer {city:'NYC'})-[:KNOWS]->(b) WHERE b.age>30 RETURN a.ext_id, b.name`

Plan:

1. Scan `graphs.<g>.nodes` for `a`

   * filter label + `{}` + per-node predicates
   * project `a__node_id`, demanded `a__ext_id`, demanded `a__props`
2. ExpandOutExec on `a__node_id`
3. Optional post-filter on type
4. Project `dst AS b__node_id` (carry forward `a__...`)
5. Join `graphs.<g>.nodes` as `b`

   * filter `b` label/spec if present
   * project demanded `b__props` (and `b__ext_id` if needed)
6. Apply graph WHERE predicates
7. RETURN projection

## 8.6 Linking external tables (via ext_id)

If RETURN/WHERE references `t.amount` from `lake.finance.transactions` linked to `a`:

* ensure `a__ext_id` is projected from nodes
* add join:

  * `JOIN lake.finance.transactions t ON t.customer_ext_id = a__ext_id`

This is standard DataFusion SQL join and works naturally with Iceberg table providers.

---

# 9) Custom DataFusion operator: `ExpandOutExec`

## 9.1 Inputs

* upstream plan
* `input_node_col: String` (e.g., `a__node_id`)
* `type_filter: Option<i32>`
* `GraphHandle { graph, snapshot_id, csr_root, n_buckets }`

## 9.2 GraphHandle resolution (Iceberg-native)

At compile time:

1. determine `edges_snapshot_id` for the query’s pinned snapshot
2. query `graphs.<g>.graph_index_manifests` for that snapshot id
3. use `csr_root` + `n_buckets`

If missing: error “CSR index missing; run build-csr”.

## 9.3 Execution algorithm

For each input RecordBatch:

1. read node ids from `input_node_col`
2. compute bucket per id and group
3. for each bucket:

   * load CSR arrays from `CSRStore` cache
   * binary search node in `srcs`
   * slice neighbors using `offsets`
   * append `(src,dst,type)` (filter inline if `type_filter` set)
4. emit large RecordBatches (~64k rows)

## 9.4 CSRStore

* cache keyed by `(graph, snapshot_id, bucket)`
* MVP: read Arrow IPC into memory
* future: mmap + zero-copy decode

---

# 10) Local MVP operational workflow

1. `init-graph <g>`: create Iceberg namespace + tables
2. load/populate:

   * nodes (must include `ext_id` for linkable labels)
   * edges
   * tokens
3. `build-csr --graph <g>` to generate CSR and write manifest row
4. Run:

   * SQL queries directly via DataFusion
   * GQL queries compiled to DataFusion plans, mixing:

     * CSR expands
     * node property filters
     * external table joins on `ext_id`

---

# 11) Future extensions (planned, ecosystem-friendly)

### A) CSC (incoming edges)

Add CSC sidecar:

* `.../snapshot=<id>/csc/bucket=...`
  Implement `ExpandInExec` reusing the same pattern.

### B) Delta + compaction (LSM adjacency)

* Base CSR/CSC + delta mini-CSR segments
* Compaction produces new base indices tied to a new snapshot
* Still registered via `graph_index_manifests` (Iceberg-native)

### C) Time travel graph queries

Expose `AT SNAPSHOT <id>`:

* pin Iceberg scans to snapshot id
* select matching CSR manifest row

### D) Uniqueness / DISTINCT frontiers

Use roaring bitmaps:

* optional `DISTINCT` semantics for k-hop to prevent blowups

### E) Declarative node-table linking (optional)

Add `graphs.<g>.node_links` to declare:

* which label maps to which external table and key column
  Compiler can then auto-infer joins.

### F) Stats tables for planning (Iceberg-native)

Persist degree histograms and label/type counts as Iceberg tables per graph.
This helps eventual cost-based planning and remains ecosystem-visible.

### G) Iceberg maintenance hooks

Add commands for:

* snapshot expiration
* manifest rewrite / compaction
  so large graphs remain performant over time.

---

# 12) MVP deliverables checklist

### Using `iceberg-datafusion` (no custom TableProvider)

* [ ] Catalog registration: DataFusion `graphs` catalog backed by Iceberg catalog provider
* [ ] Per graph tables: nodes/edges/tokens/manifests
* [ ] Token resolver (cached)
* [ ] CSR builder + manifest writer
* [ ] `ExpandOutExec` + CSRStore cache
* [ ] Compiler: AST → DataFusion plan (single chain, outgoing)
* [ ] External table joins via `ext_id` (compiler plumbing + demand projection)
* [ ] Tests: unit (lowering), operator (CSR), end-to-end (two graphs + external join)

---

## Bottom line

✅ Yes — with `ext_id` in `nodes`, the MVP supports **linking Iceberg tables to specific nodes** in a fully **DataFusion/Iceberg-native** way (just SQL joins), while keeping traversal fast with CSR and maintaining snapshot correctness through the manifest table.

If you share one concrete example of the external linkage you want (table name + join key column + which node label it attaches to), I can show the exact lowering pattern your compiler should generate (including how to project `ext_id` only when needed).
