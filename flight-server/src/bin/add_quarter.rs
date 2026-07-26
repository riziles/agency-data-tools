/// Add a new quarter's parquet file to the existing DuckLake catalog.
/// Uses streaming reads — never loads all rows into memory at once.
use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use datafusion::prelude::*;
use datafusion_ducklake::{
    DuckLakeTableWriter, SqliteMetadataWriter,
};
use futures::StreamExt;
use object_store::local::LocalFileSystem;

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "./data")]
    data_dir: PathBuf,

    #[arg(long)]
    parquet: PathBuf,

    /// Rows per DuckLake file (smaller = less memory, more files)
    #[arg(long, default_value = "500000")]
    chunk_size: usize,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let catalog_db = args.data_dir.join("catalog.db");
    let conn_str = format!("sqlite:{}?mode=rwc", catalog_db.display());

    let writer = Arc::new(SqliteMetadataWriter::new(&conn_str).await?);
    let table_writer = DuckLakeTableWriter::new(writer, Arc::new(LocalFileSystem::new()))?;

    // Read parquet with DataFusion — parquet reading is columnar, not memory-heavy
    let ctx = SessionContext::new();
    ctx.register_parquet(
        "source",
        args.parquet.to_str().unwrap(),
        ParquetReadOptions::default(),
    )
    .await?;

    // Stream in chunks instead of collecting all rows
    let mut stream = ctx.sql("SELECT * FROM source").await?.execute_stream().await?;
    let mut total_rows = 0usize;
    let mut files_written = 0usize;
    let mut chunk_batches: Vec<arrow::record_batch::RecordBatch> = Vec::new();
    let mut chunk_rows = 0usize;
    let mut last_snapshot = 0i64;

    while let Some(result) = stream.next().await {
        let batch = result?;
        let batch_rows = batch.num_rows();
        total_rows += batch_rows;
        chunk_rows += batch_rows;
        chunk_batches.push(batch);

        if chunk_rows >= args.chunk_size {
            let result = table_writer
                .append_table("main", "loans", &chunk_batches)
                .await?;
            files_written += result.files_written;
            last_snapshot = result.snapshot_id;
            println!(
                "  chunk: {} rows → snapshot {}",
                chunk_rows, result.snapshot_id
            );
            chunk_batches.clear();
            chunk_rows = 0;
        }
    }

    // Write remaining
    if !chunk_batches.is_empty() {
        let result = table_writer
            .append_table("main", "loans", &chunk_batches)
            .await?;
        files_written += result.files_written;
        last_snapshot = result.snapshot_id;
        println!(
            "  final chunk: {} rows → snapshot {}",
            chunk_rows, result.snapshot_id
        );
    }

    println!(
        "Done: {} total rows across {} files — snapshot {}",
        total_rows, files_written, last_snapshot
    );

    Ok(())
}
