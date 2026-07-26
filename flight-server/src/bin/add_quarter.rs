/// Add a new quarter's parquet file to the existing DuckLake catalog.
use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use datafusion::prelude::*;
use datafusion_ducklake::{
    DuckLakeTableWriter, MetadataProvider, SqliteMetadataProvider, SqliteMetadataWriter,
};
use object_store::local::LocalFileSystem;

#[derive(Parser)]
struct Args {
    /// Path to data directory (contains catalog.db and datalake/)
    #[arg(long, default_value = "./data")]
    data_dir: PathBuf,

    /// Parquet file to add
    #[arg(long)]
    parquet: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let catalog_db = args.data_dir.join("catalog.db");
    let conn_str = format!("sqlite:{}?mode=rwc", catalog_db.display());
    let ro_conn_str = format!("sqlite:{}", catalog_db.display());

    // Open existing catalog
    let provider = Arc::new(SqliteMetadataProvider::new(&ro_conn_str).await?);
    let current_snapshot = provider.get_current_snapshot()?;
    println!("Current snapshot: {}", current_snapshot);

    // Read new parquet
    let ctx = SessionContext::new();
    ctx.register_parquet(
        "source",
        args.parquet.to_str().unwrap(),
        ParquetReadOptions::default(),
    )
    .await?;

    let df = ctx.sql("SELECT * FROM source").await?;
    let batches: Vec<_> = df.collect().await?;
    let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
    println!("Read {} rows from {}", total_rows, args.parquet.display());

    // Append to DuckLake
    let writer = Arc::new(SqliteMetadataWriter::new(&conn_str).await?);
    let table_writer = DuckLakeTableWriter::new(writer, Arc::new(LocalFileSystem::new()))?;
    let result = table_writer
        .append_table("main", "loans", &batches)
        .await?;

    println!(
        "Appended {} rows across {} file(s) — snapshot {}",
        result.records_written, result.files_written, result.snapshot_id
    );

    Ok(())
}
