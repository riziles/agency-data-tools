use bytes::Bytes;
use datafusion::datasource::MemTable;
use datafusion::parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use datafusion::prelude::*;
use parquet::file::footer::decode_metadata;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

/// Parse raw footer bytes (Thrift FileMetaData, without the 8-byte trailer)
/// and return row group column stats as JSON.
#[wasm_bindgen]
pub fn get_row_group_stats(footer_metadata_bytes: Vec<u8>) -> Result<String, JsValue> {
    let meta = decode_metadata(&footer_metadata_bytes)
        .map_err(|e| JsValue::from_str(&format!("Footer parse: {e}")))?;
    let file_meta = meta.file_metadata();

    let mut groups = Vec::new();
    for (i, rg) in meta.row_groups().iter().enumerate() {
        // Find byte range of this row group from column offsets
        let mut min_offset = u64::MAX;
        let mut max_end = 0u64;
        let mut columns = Vec::new();

        for col in rg.columns() {
            // data_page_offset is the absolute file offset; file_offset may be relative
            let start = col.data_page_offset() as u64;
            let end = start + col.compressed_size() as u64;
            if col.compressed_size() > 0 {
                min_offset = min_offset.min(start);
                max_end = max_end.max(end);
            }

            let path = col.column_path().string();
            let mut col_info = serde_json::json!({
                "path": path,
                "offset": start,
                "length": col.compressed_size(),
            });

            // Column statistics (min/max values for predicate pushdown)
            if let Some(stats) = col.statistics() {
                if let Some(min) = stats.min_bytes_opt() {
                    col_info["min"] = serde_json::Value::String(
                        String::from_utf8_lossy(min).to_string()
                    );
                }
                if let Some(max) = stats.max_bytes_opt() {
                    col_info["max"] = serde_json::Value::String(
                        String::from_utf8_lossy(max).to_string()
                    );
                }
                if let Some(null_count) = stats.null_count_opt() {
                    col_info["null_count"] = serde_json::json!(null_count);
                }
            }
            columns.push(col_info);
        }

        groups.push(serde_json::json!({
            "index": i,
            "rows": rg.num_rows(),
            "byte_offset": min_offset,
            "byte_length": max_end - min_offset,
            "columns": columns,
        }));
    }

    let result = serde_json::json!({
        "num_rows": file_meta.num_rows(),
        "num_row_groups": meta.num_row_groups(),
        "created_by": file_meta.created_by().unwrap_or_default(),
        "row_groups": groups,
    });

    Ok(result.to_string())
}

/// Accept raw Parquet bytes and run SQL against them. Returns JSON result.
#[wasm_bindgen]
pub async fn query_parquet(parquet_bytes: Vec<u8>, sql: &str) -> Result<String, JsValue> {
    query_parquet_inner(parquet_bytes, None, sql).await
}

/// Like query_parquet but only reads specific row groups (0-indexed).
/// Used for smart fetch — skip row groups that don't match WHERE clause.
#[wasm_bindgen]
pub async fn query_parquet_rgs(parquet_bytes: Vec<u8>, rgs: Vec<usize>, sql: &str) -> Result<String, JsValue> {
    query_parquet_inner(parquet_bytes, Some(rgs), sql).await
}

async fn query_parquet_inner(parquet_bytes: Vec<u8>, rgs: Option<Vec<usize>>, sql: &str) -> Result<String, JsValue> {
    let bytes_len = parquet_bytes.len();
    let bytes = Bytes::from(parquet_bytes);

    let mut builder = ParquetRecordBatchReaderBuilder::try_new(bytes)
        .map_err(|e| JsValue::from_str(&format!("Parquet open: {e}")))?;

    // Only read specified row groups (skip zero-filled gaps)
    if let Some(indices) = &rgs {
        builder = builder.with_row_groups(indices.clone());
    }

    let schema = builder.schema().clone();
    let batches: Vec<_> = builder
        .build()
        .map_err(|e| JsValue::from_str(&format!("Build reader: {e}")))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| JsValue::from_str(&format!("Read batches: {e}")))?;

    let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();

    // Register as table and run SQL
    let ctx = SessionContext::new();
    let table = Arc::new(
        MemTable::try_new(schema, vec![batches])
            .map_err(|e| JsValue::from_str(&format!("Table: {e}")))?,
    );
    ctx.register_table("data", table)
        .map_err(|e| JsValue::from_str(&format!("Register: {e}")))?;

    let df = ctx
        .sql(sql)
        .await
        .map_err(|e| JsValue::from_str(&format!("SQL: {e}")))?;

    let results = df
        .collect()
        .await
        .map_err(|e| JsValue::from_str(&format!("Query: {e}")))?;

    let result_rows: usize = results.iter().map(|b| b.num_rows()).sum();
    let result_cols = results.first().map(|b| b.num_columns()).unwrap_or(0);

    let json = batches_to_json(&results);

    let summary = serde_json::json!({
        "parquet_bytes": bytes_len,
        "input_rows": total_rows,
        "result_rows": result_rows,
        "result_columns": result_cols,
        "data": json,
    });

    Ok(summary.to_string())
}

fn batches_to_json(
    batches: &[datafusion::arrow::record_batch::RecordBatch],
) -> Vec<serde_json::Value> {
    use datafusion::arrow::array::*;
    use datafusion::arrow::datatypes::*;

    if batches.is_empty() {
        return vec![];
    }
    let schema = &batches[0].schema();
    let names: Vec<&str> = schema.fields().iter().map(|f| f.name().as_str()).collect();
    let mut rows = Vec::new();
    for batch in batches {
        for row_idx in 0..batch.num_rows() {
            let mut obj = serde_json::Map::new();
            for (col_idx, name) in names.iter().enumerate() {
                let col = batch.column(col_idx);
                let val = col_to_json(col.as_ref(), row_idx);
                obj.insert(name.to_string(), val);
            }
            rows.push(serde_json::Value::Object(obj));
        }
    }
    rows
}

fn col_to_json(col: &dyn datafusion::arrow::array::Array, idx: usize) -> serde_json::Value {
    use datafusion::arrow::array::*;
    use datafusion::arrow::datatypes::*;

    if col.is_null(idx) {
        return serde_json::Value::Null;
    }
    match col.data_type() {
        DataType::Utf8 => serde_json::Value::String(
            col.as_any().downcast_ref::<StringArray>().unwrap().value(idx).to_string(),
        ),
        DataType::Int32 => serde_json::Value::Number(serde_json::Number::from(
            col.as_any().downcast_ref::<Int32Array>().unwrap().value(idx),
        )),
        DataType::Int64 => serde_json::Value::Number(serde_json::Number::from(
            col.as_any().downcast_ref::<Int64Array>().unwrap().value(idx),
        )),
        DataType::Float64 => {
            let v = col.as_any().downcast_ref::<Float64Array>().unwrap().value(idx);
            serde_json::Number::from_f64(v)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null)
        }
        DataType::Float32 => {
            let v = col.as_any().downcast_ref::<Float32Array>().unwrap().value(idx) as f64;
            serde_json::Number::from_f64(v)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null)
        }
        DataType::Boolean => serde_json::Value::Bool(
            col.as_any().downcast_ref::<BooleanArray>().unwrap().value(idx),
        ),
        _ => serde_json::Value::String("?".to_string()),
    }
}
