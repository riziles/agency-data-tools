use anyhow::{Context, Result};
use clap::Parser;
use datafusion::arrow::datatypes::{DataType, Field, Schema};
use datafusion::dataframe::DataFrameWriteOptions;
use datafusion::prelude::*;
use std::sync::Arc;

/// Download Fannie Mae loan performance data and convert to Parquet.
#[derive(Parser)]
#[command(name = "fannie-ingest", version)]
struct Cli {
    /// Year to fetch (e.g., 2024). Use "All" with quarter for full history.
    #[arg(short, long, default_value = "2024")]
    year: String,

    /// Quarter: Q1, Q2, Q3, Q4, or "All" for full year
    #[arg(short, long, default_value = "Q1")]
    quarter: String,

    /// Output Parquet file path
    #[arg(short, long, default_value = "output.parquet")]
    output: String,

    /// Skip the API call and just convert a local CSV to Parquet
    #[arg(long)]
    local_csv: Option<String>,

    /// R2 bucket name (optional — upload after conversion)
    #[arg(short = 'b', long)]
    r2_bucket: Option<String>,

    /// R2 object key (defaults to output filename)
    #[arg(short = 'k', long)]
    r2_key: Option<String>,
}

// Fannie Mae uses pipe-delimited CSVs with no headers.
// Full schema: see https://capitalmarkets.fanniemae.com/resources/file/credit-risk/pdf/crt-file-layout-and-glossary.pdf
fn build_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("loan_id", DataType::Utf8, false),
        Field::new("origination_date", DataType::Utf8, false),
        Field::new("original_upb", DataType::Float64, true),
        Field::new("original_interest_rate", DataType::Float64, true),
        Field::new("original_loan_term", DataType::Int32, true),
        Field::new("borrower_credit_score", DataType::Int32, true),
        Field::new("property_state", DataType::Utf8, true),
        Field::new("property_type", DataType::Utf8, true),
        Field::new("occupancy_status", DataType::Utf8, true),
        Field::new("loan_purpose", DataType::Utf8, true),
    ]))
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file (for FANNIE_CLIENT_ID, FANNIE_CLIENT_SECRET, R2_*)
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    // --- Step 1: Get the CSV (from API or local file) ---
    let csv_path = if let Some(local) = &cli.local_csv {
        local.clone()
    } else {
        println!("🔑 Authenticating with Fannie Mae API...");
        let token = get_access_token().await?;

        let year = &cli.year;
        let quarter = &cli.quarter;
        println!("📡 Fetching signed URL for {year} {quarter}...");
        let s3_url = get_signed_url(&token, year, quarter).await?;
        println!("☁️  Signed URL received");

        println!("📥 Downloading CSV...");
        let csv_bytes = reqwest::get(&s3_url).await?.bytes().await?;

        let csv_path = format!("fannie_{year}_{quarter}.csv");
        tokio::fs::write(&csv_path, &csv_bytes).await?;
        println!("✅ Downloaded {csv_path} ({} MB)", csv_bytes.len() / 1_048_576);
        csv_path
    };

    // --- Step 2: Convert CSV to Parquet ---
    let ctx = SessionContext::new();
    let schema = build_schema();

    println!("📄 Reading CSV: {csv_path}");
    let csv_options = CsvReadOptions::new()
        .has_header(false)
        .schema(&schema)
        .delimiter(b'|');

    let df = ctx.read_csv(&csv_path, csv_options).await?;
    let row_count = df.clone().count().await?;
    println!("✅ Read {row_count} rows");

    println!("💾 Writing Parquet: {}", cli.output);
    df.write_parquet(&cli.output, DataFrameWriteOptions::new(), None).await?;
    println!("✅ Parquet written ({} rows)", row_count);

    // --- Step 3: Upload to R2 if requested ---
    if let Some(bucket) = cli.r2_bucket {
        let key = cli.r2_key.unwrap_or_else(|| cli.output.clone());
        println!("☁️  Uploading to R2: {bucket}/{key}");
        upload_to_r2(&cli.output, &bucket, &key).await?;
        println!("✅ Uploaded");
    }

    Ok(())
}

/// Get an OAuth2 access token from Fannie Mae using client credentials.
async fn get_access_token() -> Result<String> {
    let client_id = std::env::var("FANNIE_CLIENT_ID")
        .context("FANNIE_CLIENT_ID not set")?;
    let client_secret = std::env::var("FANNIE_CLIENT_SECRET")
        .context("FANNIE_CLIENT_SECRET not set")?;

    let client = reqwest::Client::new();
    let resp = client
        .post("https://fmsso-prod.fanniemae.com/as/token.oauth2")
        .basic_auth(&client_id, Some(&client_secret))
        .form(&[("grant_type", "client_credentials")])
        .send()
        .await?;

    let status = resp.status();
    let body: serde_json::Value = resp.json().await?;

    if !status.is_success() {
        anyhow::bail!("Auth failed ({status}): {body}");
    }

    let token = body["access_token"]
        .as_str()
        .context("No access_token in response")?
        .to_string();

    Ok(token)
}

/// Get a signed S3 URL for a specific year/quarter.
async fn get_signed_url(token: &str, year: &str, quarter: &str) -> Result<String> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://api.fanniemae.com/v1/sf-loan-performance-data/years/{year}/quarters/{quarter}"
    );

    let resp = client
        .get(&url)
        .header("x-public-access-token", token)
        .header("Content-Type", "application/json")
        .send()
        .await?;

    let status = resp.status();
    let body: serde_json::Value = resp.json().await?;

    if !status.is_success() {
        anyhow::bail!("API error ({status}): {body}");
    }

    // Response is { "lphResponse": [{ "s3Uri": "https://...", "year": ..., "quarter": "..." }] }
    let s3_uri = body["lphResponse"][0]["s3Uri"]
        .as_str()
        .or_else(|| body["s3Uri"].as_str())
        .context("No s3Uri in response")?
        .to_string();

    Ok(s3_uri)
}

/// Upload a file to Cloudflare R2.
async fn upload_to_r2(path: &str, bucket: &str, key: &str) -> Result<()> {
    use object_store::aws::AmazonS3Builder;
    use object_store::{ObjectStore, PutPayload};

    let account_id = std::env::var("R2_ACCOUNT_ID")
        .context("R2_ACCOUNT_ID not set")?;

    let r2 = AmazonS3Builder::new()
        .with_endpoint(format!("https://{account_id}.r2.cloudflarestorage.com"))
        .with_region("auto")
        .with_bucket_name(bucket)
        .with_access_key_id(
            &std::env::var("R2_ACCESS_KEY_ID").context("R2_ACCESS_KEY_ID not set")?,
        )
        .with_secret_access_key(
            &std::env::var("R2_SECRET_ACCESS_KEY")
                .context("R2_SECRET_ACCESS_KEY not set")?,
        )
        .build()?;

    let data = tokio::fs::read(path).await?;
    r2.put(
        &object_store::path::Path::from(key),
        PutPayload::from_bytes(data.into()),
    )
    .await?;

    Ok(())
}
