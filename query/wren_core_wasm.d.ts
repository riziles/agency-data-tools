/* tslint:disable */
/* eslint-disable */
/**
 * The `ReadableStreamType` enum.
 *
 * *This API requires the following crate features to be activated: `ReadableStreamType`*
 */

type ReadableStreamType = "bytes";

export class IntoUnderlyingByteSource {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    cancel(): void;
    pull(controller: ReadableByteStreamController): Promise<any>;
    start(controller: ReadableByteStreamController): void;
    readonly autoAllocateChunkSize: number;
    readonly type: ReadableStreamType;
}

export class IntoUnderlyingSink {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    abort(reason: any): Promise<any>;
    close(): Promise<any>;
    write(chunk: any): Promise<any>;
}

export class IntoUnderlyingSource {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    cancel(): void;
    pull(controller: ReadableStreamDefaultController): Promise<any>;
}

/**
 * Wren Engine WASM instance.
 *
 * Holds a DataFusion SessionContext and (after `loadMDL`) the analyzed
 * MDL. `analyzed_mdl` is kept so the cube API (`cubeQuery`, `listCubes`)
 * can read the manifest after `loadMDL` returns. All query execution
 * happens in-browser via DataFusion.
 */
export class WrenEngine {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Execute a structured CubeQuery against the loaded MDL.
     *
     * Takes a JSON-encoded `CubeQuery` (matching the camelCase shape used
     * by the Python binding), translates it to SQL via wren-core, and
     * runs the SQL through the existing `query()` path. Returns a JSON
     * array of result rows.
     *
     * Requires `loadMDL` to have been called first.
     */
    cubeQuery(cube_query_json: string): Promise<string>;
    /**
     * List the cubes defined in the loaded MDL.
     *
     * Returns a JSON array of `{ name, baseObject, measures, dimensions,
     * timeDimensions, hierarchies }` records. Requires `loadMDL` to have
     * been called first.
     */
    listCubes(): string;
    /**
     * Load an MDL (Modeling Definition Language) manifest.
     *
     * Parses the MDL JSON, builds the semantic layer (AnalyzedWrenMDL),
     * and reconfigures the SessionContext with Wren analyzer rules in
     * LocalRuntime mode (direct DataFusion execution, no SQL generation).
     *
     * The `source` parameter selects how physical tables are resolved:
     *
     * - `http://…/`, `https://…/` → **URL mode**. For each model, registers a
     *   DataFusion `ListingTable` at `{source}/{table_name}.parquet`. Tables
     *   do not need pre-registering. (`s3://` and `gs://` schemes are Phase 4
     *   and fall through to local mode today.)
     * - `""` (empty) → **fallback mode**: the M3+ behaviour of auto-detecting
     *   URL vs local tables from each model's `tableReference`. Preserved for
     *   backwards compatibility with MDLs that still embed URLs in
     *   `tableReference`.
     * - anything else → **local mode**. The caller is expected to have
     *   pre-registered each model's physical table via
     *   `registerParquet`/`registerJson`. If any model's physical table is
     *   missing, `loadMDL` returns an `Unresolved models: [...]` error up
     *   front instead of deferring to query time.
     *
     * After loading, bare model names resolve under the MDL's catalog/schema
     * (typically `wren.public`), so queries can reference models without a
     * catalog prefix.
     */
    loadMDL(mdl_json: string, source: string): Promise<void>;
    /**
     * Initialize a new WrenEngine instance.
     *
     * Creates a DataFusion SessionContext with default configuration
     * suitable for single-threaded WASM execution. The session time zone
     * defaults to UTC (`+00:00`) so browser timestamp inference and
     * comparisons match `create_wren_ctx` on the native side.
     */
    constructor();
    /**
     * Execute a SQL query and return results as a JSON string.
     *
     * Returns a JSON array of objects, e.g. `[{"count":42,"avg":3.14},...]`
     *
     * The body runs via `runtime.block_on(...)` so DataFusion's
     * `tokio::task::spawn` calls (e.g. inside `CoalescePartitionsExec`,
     * which any multi-partition plan such as `UNION ALL` flows through)
     * see a live tokio scheduler. Without this wrapper the inner spawn
     * panics with `there is no reactor running`, surfacing in JS as
     * `RuntimeError: unreachable`.
     */
    query(sql: string): Promise<string>;
    /**
     * Register an in-memory table from CSV bytes.
     *
     * CSV is read with `arrow::csv::ReaderBuilder`. Schema is inferred from
     * the first `inferRows` rows (default 1000) unless an explicit schema is
     * provided in `options.schema`.
     *
     * # Arguments
     * * `table_name` - Name to register the table under
     * * `data` - CSV bytes
     * * `options_json` - Optional JSON-encoded `CsvReadOptions`. Empty / `""`
     *   uses defaults (header on, comma delimiter, double-quote, batch 8192).
     *
     * # Options shape (camelCase)
     * ```json
     * {
     *   "header": true,
     *   "delimiter": ",",
     *   "quote": "\"",
     *   "escape": "\\",
     *   "terminator": "\n",
     *   "batchSize": 8192,
     *   "inferRows": 1000,
     *   "schema": [
     *     {"name": "id", "type": "int64"},
     *     {"name": "amount", "type": "float64"}
     *   ]
     * }
     * ```
     * All fields are optional. Single-character options (delimiter/quote/…)
     * take only the first byte of the supplied string.
     */
    registerCsv(table_name: string, data: Uint8Array, options_json: string): Promise<void>;
    /**
     * Register an in-memory table from a JSON array of objects.
     *
     * This is a convenience method for M1 testing. In M2+, use
     * `register_parquet` to load Parquet files from the browser.
     *
     * # Arguments
     * * `table_name` - Name to register the table under
     * * `json_data` - JSON string: array of objects, e.g. `[{"a":1,"b":"x"},...]`
     */
    registerJson(table_name: string, json_data: string): Promise<void>;
    /**
     * Register a Parquet file from bytes uploaded via JS.
     *
     * Reads the Parquet data into Arrow RecordBatches and registers as a MemTable.
     * The JS side should pass the file contents as a `Uint8Array`.
     */
    registerParquet(table_name: string, data: Uint8Array): Promise<void>;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_wrenengine_free: (a: number, b: number) => void;
    readonly wrenengine_cubeQuery: (a: number, b: number, c: number) => any;
    readonly wrenengine_listCubes: (a: number) => [number, number, number, number];
    readonly wrenengine_loadMDL: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly wrenengine_new: () => [number, number, number];
    readonly wrenengine_query: (a: number, b: number, c: number) => any;
    readonly wrenengine_registerCsv: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => any;
    readonly wrenengine_registerJson: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly wrenengine_registerParquet: (a: number, b: number, c: number, d: number, e: number) => any;
    readonly rust_zstd_wasm_shim_calloc: (a: number, b: number) => number;
    readonly rust_zstd_wasm_shim_free: (a: number) => void;
    readonly rust_zstd_wasm_shim_malloc: (a: number) => number;
    readonly rust_zstd_wasm_shim_memcmp: (a: number, b: number, c: number) => number;
    readonly rust_zstd_wasm_shim_memcpy: (a: number, b: number, c: number) => number;
    readonly rust_zstd_wasm_shim_memmove: (a: number, b: number, c: number) => number;
    readonly rust_zstd_wasm_shim_memset: (a: number, b: number, c: number) => number;
    readonly rust_zstd_wasm_shim_qsort: (a: number, b: number, c: number, d: number) => void;
    readonly ring_core_0_17_14__bn_mul_mont: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly __wbg_intounderlyingbytesource_free: (a: number, b: number) => void;
    readonly __wbg_intounderlyingsource_free: (a: number, b: number) => void;
    readonly intounderlyingbytesource_autoAllocateChunkSize: (a: number) => number;
    readonly intounderlyingbytesource_cancel: (a: number) => void;
    readonly intounderlyingbytesource_pull: (a: number, b: any) => any;
    readonly intounderlyingbytesource_start: (a: number, b: any) => void;
    readonly intounderlyingbytesource_type: (a: number) => number;
    readonly intounderlyingsource_cancel: (a: number) => void;
    readonly intounderlyingsource_pull: (a: number, b: any) => any;
    readonly __wbg_intounderlyingsink_free: (a: number, b: number) => void;
    readonly intounderlyingsink_abort: (a: number, b: any) => any;
    readonly intounderlyingsink_close: (a: number) => any;
    readonly intounderlyingsink_write: (a: number, b: any) => any;
    readonly wasm_bindgen_f59c328abe25ccab___convert__closures_____invoke___wasm_bindgen_f59c328abe25ccab___JsValue__core_7d5f0a2ba6a62c33___result__Result_____wasm_bindgen_f59c328abe25ccab___JsError___true_: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen_f59c328abe25ccab___convert__closures_____invoke___js_sys_697db3c07b3a1b93___Function_fn_wasm_bindgen_f59c328abe25ccab___JsValue_____wasm_bindgen_f59c328abe25ccab___sys__Undefined___js_sys_697db3c07b3a1b93___Function_fn_wasm_bindgen_f59c328abe25ccab___JsValue_____wasm_bindgen_f59c328abe25ccab___sys__Undefined_______true_: (a: number, b: number, c: any, d: any) => void;
    readonly wasm_bindgen_f59c328abe25ccab___convert__closures_____invoke___wasm_bindgen_f59c328abe25ccab___JsValue______true_: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_f59c328abe25ccab___convert__closures_____invoke_______true_: (a: number, b: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_destroy_closure: (a: number, b: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
