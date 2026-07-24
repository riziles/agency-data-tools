use anyhow::{Context, Result};
use clap::Parser;
use datafusion::arrow::datatypes::{DataType, Field, Schema};
use datafusion::dataframe::DataFrameWriteOptions;
use datafusion::prelude::*;
use std::path::Path;
use std::sync::Arc;

/// Download Fannie Mae loan performance data and convert to Parquet.
#[derive(Parser)]
#[command(name = "fannie-ingest", version)]
struct Cli {
    /// Year to fetch (e.g., 2024)
    #[arg(short, long, default_value = "2024")]
    year: String,

    /// Quarter: Q1, Q2, Q3, Q4
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

    /// Sort Parquet output by this column (improves row-group pruning in smart fetch)
    #[arg(long, default_value = "property_state")]
    sort_by: String,
}

/// Build the 108-field schema from the Fannie Mae CRT File Layout (Oct 2020+ single-file format).
/// Includes 1 leading padding field and 4 trailing padding fields to handle pipe-delimited format
/// where lines start and end with `|`, producing 113 pipe-separated values.
fn build_schema() -> Arc<Schema> {
    let fields: Vec<Field> = vec![
        // Leading pipe creates an empty column — pad it
        Field::new("padding_lead", DataType::Utf8, true),
        // --- 108 data fields from CRT File Layout (SF data omits position 1: Reference Pool ID) ---
        Field::new("loan_id", DataType::Utf8, true),
        Field::new("monthly_reporting_period", DataType::Utf8, true),
        Field::new("channel", DataType::Utf8, true),
        Field::new("seller_name", DataType::Utf8, true),
        Field::new("servicer_name", DataType::Utf8, true),
        Field::new("master_servicer", DataType::Utf8, true),
        Field::new("original_interest_rate", DataType::Float64, true),
        Field::new("current_interest_rate", DataType::Float64, true),
        Field::new("original_upb", DataType::Float64, true),
        Field::new("upb_at_issuance", DataType::Float64, true),
        Field::new("current_actual_upb", DataType::Float64, true),
        Field::new("original_loan_term", DataType::Int32, true),
        Field::new("origination_date", DataType::Utf8, true),
        Field::new("first_payment_date", DataType::Utf8, true),
        Field::new("loan_age", DataType::Int32, true),
        Field::new("remaining_months_to_legal_maturity", DataType::Int32, true),
        Field::new("remaining_months_to_maturity", DataType::Int32, true),
        Field::new("maturity_date", DataType::Utf8, true),
        Field::new("original_ltv", DataType::Int32, true),
        Field::new("original_cltv", DataType::Int32, true),
        Field::new("number_of_borrowers", DataType::Int32, true),
        Field::new("dti", DataType::Int32, true),
        Field::new("borrower_credit_score_at_origination", DataType::Int32, true),
        Field::new("co_borrower_credit_score_at_origination", DataType::Int32, true),
        Field::new("first_time_home_buyer_indicator", DataType::Utf8, true),
        Field::new("loan_purpose", DataType::Utf8, true),
        Field::new("property_type", DataType::Utf8, true),
        Field::new("number_of_units", DataType::Int32, true),
        Field::new("occupancy_status", DataType::Utf8, true),
        Field::new("property_state", DataType::Utf8, true),
        Field::new("msa", DataType::Utf8, true),
        Field::new("zip_code_short", DataType::Utf8, true),
        Field::new("mortgage_insurance_percentage", DataType::Float64, true),
        Field::new("amortization_type", DataType::Utf8, true),
        Field::new("prepayment_penalty_indicator", DataType::Utf8, true),
        Field::new("interest_only_loan_indicator", DataType::Utf8, true),
        Field::new("interest_only_first_pi_payment_date", DataType::Utf8, true),
        Field::new("months_to_amortization", DataType::Int32, true),
        Field::new("current_loan_delinquency_status", DataType::Utf8, true),
        Field::new("loan_payment_history", DataType::Utf8, true),
        Field::new("modification_flag", DataType::Utf8, true),
        Field::new("mortgage_insurance_cancellation_indicator", DataType::Utf8, true),
        Field::new("zero_balance_code", DataType::Utf8, true),
        Field::new("zero_balance_effective_date", DataType::Utf8, true),
        Field::new("upb_at_removal", DataType::Float64, true),
        Field::new("repurchase_date", DataType::Utf8, true),
        Field::new("scheduled_principal_current", DataType::Float64, true),
        Field::new("total_principal_current", DataType::Float64, true),
        Field::new("unscheduled_principal_current", DataType::Float64, true),
        Field::new("last_paid_installment_date", DataType::Utf8, true),
        Field::new("foreclosure_date", DataType::Utf8, true),
        Field::new("disposition_date", DataType::Utf8, true),
        Field::new("foreclosure_costs", DataType::Float64, true),
        Field::new("property_preservation_and_repair_costs", DataType::Float64, true),
        Field::new("asset_recovery_costs", DataType::Float64, true),
        Field::new("misc_holding_expenses_and_credits", DataType::Float64, true),
        Field::new("associated_taxes_for_holding_property", DataType::Float64, true),
        Field::new("net_sales_proceeds", DataType::Float64, true),
        Field::new("credit_enhancement_proceeds", DataType::Float64, true),
        Field::new("repurchase_make_whole_proceeds", DataType::Float64, true),
        Field::new("other_foreclosure_proceeds", DataType::Float64, true),
        Field::new("modification_non_interest_bearing_upb", DataType::Float64, true),
        Field::new("principal_forgiveness_amount", DataType::Float64, true),
        Field::new("original_list_start_date", DataType::Utf8, true),
        Field::new("original_list_price", DataType::Float64, true),
        Field::new("current_list_start_date", DataType::Utf8, true),
        Field::new("current_list_price", DataType::Float64, true),
        Field::new("borrower_credit_score_at_issuance", DataType::Int32, true),
        Field::new("co_borrower_credit_score_at_issuance", DataType::Int32, true),
        Field::new("borrower_credit_score_current", DataType::Int32, true),
        Field::new("co_borrower_credit_score_current", DataType::Int32, true),
        Field::new("mortgage_insurance_type", DataType::Utf8, true),
        Field::new("servicing_activity_indicator", DataType::Utf8, true),
        Field::new("current_period_modification_loss_amount", DataType::Float64, true),
        Field::new("cumulative_modification_loss_amount", DataType::Float64, true),
        Field::new("current_period_credit_event_net_gain_or_loss", DataType::Float64, true),
        Field::new("cumulative_credit_event_net_gain_or_loss", DataType::Float64, true),
        Field::new("special_eligibility_program", DataType::Utf8, true),
        Field::new("foreclosure_principal_write_off_amount", DataType::Float64, true),
        Field::new("relocation_mortgage_indicator", DataType::Utf8, true),
        Field::new("zero_balance_code_change_date", DataType::Utf8, true),
        Field::new("loan_holdback_indicator", DataType::Utf8, true),
        Field::new("loan_holdback_effective_date", DataType::Utf8, true),
        Field::new("delinquent_accrued_interest", DataType::Float64, true),
        Field::new("property_valuation_method", DataType::Utf8, true),
        Field::new("high_balance_loan_indicator", DataType::Utf8, true),
        Field::new("arm_initial_fixed_rate_period_le_5yr_indicator", DataType::Utf8, true),
        Field::new("arm_product_type", DataType::Utf8, true),
        Field::new("initial_fixed_rate_period", DataType::Int32, true),
        Field::new("interest_rate_adjustment_frequency", DataType::Int32, true),
        Field::new("next_interest_rate_adjustment_date", DataType::Utf8, true),
        Field::new("next_payment_change_date", DataType::Utf8, true),
        Field::new("arm_index", DataType::Utf8, true),
        Field::new("arm_cap_structure", DataType::Utf8, true),
        Field::new("initial_interest_rate_cap_up_percent", DataType::Float64, true),
        Field::new("periodic_interest_rate_cap_up_percent", DataType::Float64, true),
        Field::new("lifetime_interest_rate_cap_up_percent", DataType::Float64, true),
        Field::new("mortgage_margin", DataType::Float64, true),
        Field::new("arm_balloon_indicator", DataType::Utf8, true),
        Field::new("arm_plan_number", DataType::Int32, true),
        Field::new("borrower_assistance_plan", DataType::Utf8, true),
        Field::new("hltv_refinance_option_indicator", DataType::Utf8, true),
        Field::new("deal_name", DataType::Utf8, true),
        Field::new("repurchase_make_whole_proceeds_flag", DataType::Utf8, true),
        Field::new("alternative_delinquency_resolution", DataType::Utf8, true),
        Field::new("alternative_delinquency_resolution_count", DataType::Int32, true),
        Field::new("total_deferral_amount", DataType::Float64, true),
        // Extra field beyond the June 2023 PDF (appears in 2024 Q1 data)
        Field::new("extra_field_109", DataType::Utf8, true),
        // Trailing empty fields from trailing pipes
        Field::new("padding_trail_1", DataType::Utf8, true),
        Field::new("padding_trail_2", DataType::Utf8, true),
        Field::new("padding_trail_3", DataType::Utf8, true),
        Field::new("padding_trail_4", DataType::Utf8, true),
    ];
    Arc::new(Schema::new(fields))
}

#[tokio::main]
async fn main() -> Result<()> {
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
        let zip_path = format!("fannie_{year}_{quarter}.zip");
        let csv_path = format!("fannie_{year}_{quarter}.csv");

        // Skip download+extract if CSV already cached on disk
        if Path::new(&csv_path).exists() {
            println!("📦 Using cached CSV: {csv_path}");
        } else {
            println!("📡 Fetching signed URL for {year} {quarter}...");
            let s3_url = get_signed_url(&token, year, quarter).await?;
            println!("☁️  Signed URL received");

            if Path::new(&zip_path).exists() {
                println!("📥 Using cached ZIP: {zip_path}");
            } else {
                println!("📥 Downloading ZIP...");
                let zip_bytes = reqwest::get(&s3_url).await?.bytes().await?;
                tokio::fs::write(&zip_path, &zip_bytes).await?;
                let size_mb = zip_bytes.len() / 1_048_576;
                println!("✅ Downloaded {zip_path} ({size_mb} MB)");
            }

            println!("📦 Extracting CSV from ZIP...");
            extract_csv_from_zip(&zip_path, &csv_path)?;
            println!("✅ Extracted to {csv_path}");
        }
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

    println!("📊 Sorting by: {}", cli.sort_by);
    let df = df.sort_by(vec![col(&cli.sort_by)])?;
    println!("✅ Sorted");

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

/// Extract the first .csv file from a ZIP archive.
fn extract_csv_from_zip(zip_path: &str, output_csv: &str) -> Result<()> {
    let zip_file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(zip_file)?;

    // Find the first .csv file in the archive
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        if name.ends_with(".csv") {
            let out_path = Path::new(output_csv);
            let mut output = std::fs::File::create(out_path)?;
            std::io::copy(&mut std::io::BufReader::new(entry), &mut output)?;
            return Ok(());
        }
    }
    anyhow::bail!("No .csv file found in ZIP archive")
}

/// Get an OAuth2 access token from Fannie Mae using client credentials.
async fn get_access_token() -> Result<String> {
    let client_id =
        std::env::var("FANNIE_CLIENT_ID").context("FANNIE_CLIENT_ID not set")?;
    let client_secret =
        std::env::var("FANNIE_CLIENT_SECRET").context("FANNIE_CLIENT_SECRET not set")?;

    let client = reqwest::Client::new();
    let resp = client
        .post("https://auth.pingone.com/4c2b23f9-52b1-4f8f-aa1f-1d477590770c/as/token")
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
        .header("Accept", "application/json")
        .send()
        .await?;

    let status = resp.status();
    let body: serde_json::Value = resp.json().await?;

    if !status.is_success() {
        anyhow::bail!("API error ({status}): {body}");
    }

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

    let account_id = std::env::var("R2_ACCOUNT_ID").context("R2_ACCOUNT_ID not set")?;

    let r2 = AmazonS3Builder::new()
        .with_endpoint(format!(
            "https://{account_id}.r2.cloudflarestorage.com"
        ))
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
