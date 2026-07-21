use std::{path::PathBuf, sync::Arc};

use anyhow::{Context, Result};
use arrow::record_batch::RecordBatch;
use arrow_flight::flight_service_server::FlightServiceServer;
use clap::Parser;
use datafusion::prelude::*;
use datafusion_ducklake::{
    DuckLakeCatalog, DuckLakeTableWriter, MetadataProvider, MetadataWriter,
    SqliteMetadataProvider, SqliteMetadataWriter, register_ducklake_functions,
};
use datafusion_flight_sql_server::service::FlightSqlService;
use object_store::local::LocalFileSystem;
use tonic_web::GrpcWebLayer;
use tower_http::cors::CorsLayer;

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "127.0.0.1:50051")]
    bind: String,

    #[arg(long, default_value = "./data")]
    data_dir: PathBuf,

    #[arg(long, default_value = "./data/2024Q1.parquet")]
    parquet: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    std::fs::create_dir_all(&args.data_dir)?;

    let catalog_db = args.data_dir.join("catalog.db");
    let conn_str = format!("sqlite:{}?mode=rwc", catalog_db.display());
    let ro_conn_str = format!("sqlite:{}", catalog_db.display());

    // --- Phase 1: Initialize DuckLake catalog ---
    if !catalog_db.exists() {
        println!("Initializing DuckLake catalog at {}", catalog_db.display());
        initialize_catalog(&conn_str, &args).await?;
    } else {
        println!("Using existing DuckLake catalog at {}", catalog_db.display());
    }

    // --- Phase 2: Open catalog and register with DataFusion ---
    let provider = Arc::new(SqliteMetadataProvider::new(&ro_conn_str).await?);
    let snapshot_id = provider.get_current_snapshot()?;
    println!("Catalog snapshot: {}", snapshot_id);

    let ducklake_catalog = DuckLakeCatalog::with_snapshot(provider.clone(), snapshot_id)?;
    let config = SessionConfig::new().with_default_catalog_and_schema("ducklake", "main");
    let ctx = SessionContext::new_with_config(config);

    ctx.register_catalog("ducklake", Arc::new(ducklake_catalog));
    register_ducklake_functions(&ctx, provider);

    if let Some(cat) = ctx.catalog("ducklake") {
        for s in cat.schema_names() {
            if let Some(schema) = cat.schema(&s) {
                println!("Schema '{}': tables = {:?}", s, schema.table_names());
            }
        }
    }

    // --- Phase 3: Serve Flight SQL ---
    let addr = args.bind.parse().context("invalid bind address")?;
    println!("Starting Flight SQL server on {}", args.bind);

    let service = FlightSqlService::new(ctx.state());
    let svc = FlightServiceServer::new(service);

    tonic::transport::Server::builder()
        .accept_http1(true)
        .layer(CorsLayer::permissive())
        .layer(GrpcWebLayer::new())
        .add_service(svc)
        .serve(addr)
        .await?;

    Ok(())
}

async fn initialize_catalog(conn_str: &str, args: &Args) -> Result<()> {
    let lake_data_dir = args.data_dir.join("datalake");
    std::fs::create_dir_all(&lake_data_dir)?;

    let writer = Arc::new(SqliteMetadataWriter::new_with_init(conn_str).await?);
    writer.set_data_path(lake_data_dir.to_str().unwrap())?;

    // Read source Parquet
    println!("Reading {}", args.parquet.display());
    let reader_ctx = SessionContext::new();
    reader_ctx
        .register_parquet(
            "source",
            args.parquet.to_str().unwrap(),
            ParquetReadOptions::default(),
        )
        .await?;

    let df = reader_ctx.sql("SELECT * FROM source").await?;
    let batches: Vec<RecordBatch> = df.collect().await?;
    let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
    println!("Read {} rows in {} batches", total_rows, batches.len());

    // Write into DuckLake
    println!("Writing {} rows to DuckLake...", total_rows);
    let table_writer = DuckLakeTableWriter::new(
        Arc::new(SqliteMetadataWriter::new(conn_str).await?),
        Arc::new(LocalFileSystem::new()),
    )?;

    let result = table_writer
        .write_table("main", "loans", &batches)
        .await?;
    println!(
        "Wrote {} rows across {} file(s) — snapshot {}, table {}",
        result.records_written, result.files_written, result.snapshot_id, result.table_id
    );

    Ok(())
}
