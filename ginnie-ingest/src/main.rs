use anyhow::{Context, Result};
use clap::Parser;
use datafusion::arrow::datatypes::{DataType, Field, Schema};
use datafusion::prelude::*;
use datafusion::dataframe::DataFrameWriteOptions;
use datafusion::prelude::CsvReadOptions;
use std::path::Path;
use std::sync::Arc;

/// Download Ginnie Mae Loan Performance data and convert to Parquet.
#[derive(Parser)]
#[command(name = "ginnie-ingest", version)]
struct Cli {
    /// Year to fetch (e.g., 2023)
    #[arg(short, long, default_value = "2023")]
    year: String,

    /// Quarter: Q1, Q2, Q3, Q4
    #[arg(short, long, default_value = "Q2")]
    quarter: String,

    /// Output Parquet file path
    #[arg(short, long, default_value = "ginnie_output.parquet")]
    output: String,

    /// Path to cookie file (default: secrets/ginnie-cookies.txt)
    #[arg(short = 'c', long, default_value = "secrets/ginnie-cookies.txt")]
    cookie_file: String,

    /// Skip download, convert local text file instead
    #[arg(long)]
    local_file: Option<String>,
}

/// Build the 60-field Ginnie Mae Loan Performance schema (LP-01 through LP-59 + trailing padding).
/// Source: Loan Performance File Layout V1.0 (May 2023)
fn build_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        // LP-01: Record Type = "LP" (Character 2)
        Field::new("record_type", DataType::Utf8, true),
        // LP-02: CUSIP Number (Character 9)
        Field::new("cusip", DataType::Utf8, true),
        // LP-03: Pool ID (Character 6)
        Field::new("pool_id", DataType::Utf8, true),
        // LP-04: Pool Indicator — X, C, or M (Character 1)
        Field::new("pool_indicator", DataType::Utf8, true),
        // LP-05: Pool Type (Character 2)
        Field::new("pool_type", DataType::Utf8, true),
        // LP-06: Pool Issue Date (Numeric 8, CCYYMMDD)
        Field::new("pool_issue_date", DataType::Utf8, true),
        // LP-07: Disclosure Sequence Number (Numeric 10)
        Field::new("disclosure_seq_num", DataType::Utf8, true),
        // LP-08: Issuer ID (Numeric 4)
        Field::new("issuer_id", DataType::Utf8, true),
        // LP-09: Agency — F/V/R/N/9 (Character 1)
        Field::new("agency", DataType::Utf8, true),
        // LP-10: Loan Purpose — 1-5,9 (Numeric 1)
        Field::new("loan_purpose", DataType::Int32, true),
        // LP-11: Refinance Type (Numeric 1)
        Field::new("refinance_type", DataType::Int32, true),
        // LP-12: First Payment Date (Numeric 8, CCYYMMDD)
        Field::new("first_payment_date", DataType::Utf8, true),
        // LP-13: Maturity Date (Numeric 8, CCYYMMDD)
        Field::new("maturity_date", DataType::Utf8, true),
        // LP-14: Loan Interest Rate (Numeric 5, 3.2)
        Field::new("interest_rate", DataType::Float64, true),
        // LP-15: Original Principal Balance (Numeric 15, 13.2)
        Field::new("original_principal_balance", DataType::Float64, true),
        // LP-16: UPB at Issuance (Numeric 15, 13.2)
        Field::new("upb_at_issuance", DataType::Float64, true),
        // LP-17: Unpaid Principal Balance (Numeric 15, 13.2)
        Field::new("unpaid_principal_balance", DataType::Float64, true),
        // LP-18: Original Loan Term, Months (Numeric 3)
        Field::new("original_loan_term", DataType::Int32, true),
        // LP-19: Loan Age, Months (Numeric 3)
        Field::new("loan_age", DataType::Int32, true),
        // LP-20: Remaining Loan Term, Months (Numeric 3)
        Field::new("remaining_term", DataType::Int32, true),
        // LP-21: Months Delinquent (Numeric 1)
        Field::new("months_delinquent", DataType::Int32, true),
        // LP-22: Months Pre-Paid (Numeric 1)
        Field::new("months_prepaid", DataType::Int32, true),
        // LP-23: Loan Gross Margin — ARM only (Numeric 4, 1.3)
        Field::new("gross_margin", DataType::Float64, true),
        // LP-24: Loan To Value (Numeric 5, 3.2)
        Field::new("ltv", DataType::Float64, true),
        // LP-25: Combined LTV (Numeric 5, 3.2)
        Field::new("cltv", DataType::Float64, true),
        // LP-26: Total Debt Expense Ratio (Numeric 5, 3.2)
        Field::new("dti", DataType::Float64, true),
        // LP-27: Credit Score (Numeric 3)
        Field::new("credit_score", DataType::Int32, true),
        // LP-28: Down Payment Assistance — Y/N (Character 1)
        Field::new("down_payment_assistance", DataType::Utf8, true),
        // LP-29: Buy Down Status — Y/N (Character 1)
        Field::new("buydown_status", DataType::Utf8, true),
        // LP-30: Upfront MIP rate (Numeric 5, 2.3)
        Field::new("upfront_mip", DataType::Float64, true),
        // LP-31: Annual MIP rate (Numeric 5, 2.3)
        Field::new("annual_mip", DataType::Float64, true),
        // LP-32: Number of Borrowers (Numeric 1)
        Field::new("num_borrowers", DataType::Int32, true),
        // LP-33: First Time Home Buyer — Y/N (Character 1)
        Field::new("first_time_buyer", DataType::Utf8, true),
        // LP-34: Property Type / Number of Living Units (Numeric 1)
        Field::new("num_units", DataType::Int32, true),
        // LP-35: State — 2-char code (Character 2)
        Field::new("property_state", DataType::Utf8, true),
        // LP-36: MSA code (Numeric 5)
        Field::new("msa", DataType::Utf8, true),
        // LP-37: Third-Party Origination Type (Numeric 1)
        Field::new("third_party_origination", DataType::Int32, true),
        // LP-38: Current Month Liquidation Flag — Y/N (Character 1)
        Field::new("liquidation_flag", DataType::Utf8, true),
        // LP-39: Removal Reason (Numeric 1)
        Field::new("removal_reason", DataType::Int32, true),
        // LP-40: As of Date / Reporting Period (Numeric 6, CCYYMM)
        Field::new("as_of_date", DataType::Utf8, true),
        // LP-41: Loan Origination Date (Numeric 8, CCYYMMDD)
        Field::new("origination_date", DataType::Utf8, true),
        // LP-42: Seller Issuer ID (Numeric 4)
        Field::new("seller_issuer_id", DataType::Utf8, true),
        // LP-43: Index Type — CMT/LIBOR (Character 5)
        Field::new("index_type", DataType::Utf8, true),
        // LP-44: Look-Back Period (Numeric 2)
        Field::new("lookback_period", DataType::Int32, true),
        // LP-45: Interest Rate Change Date (Numeric 8)
        Field::new("rate_change_date", DataType::Utf8, true),
        // LP-46: Initial Interest Rate Cap (Numeric 1)
        Field::new("initial_rate_cap", DataType::Int32, true),
        // LP-47: Subsequent Interest Rate Cap (Numeric 1)
        Field::new("subsequent_rate_cap", DataType::Int32, true),
        // LP-48: Lifetime Interest Rate Cap (Numeric 1)
        Field::new("lifetime_rate_cap", DataType::Int32, true),
        // LP-49: Next Rate Change Ceiling (Numeric 5, 3.2)
        Field::new("next_rate_ceiling", DataType::Float64, true),
        // LP-50: Lifetime Rate Ceiling (Numeric 5, 3.2)
        Field::new("lifetime_rate_ceiling", DataType::Float64, true),
        // LP-51: Lifetime Rate Floor (Numeric 5, 3.2)
        Field::new("lifetime_rate_floor", DataType::Float64, true),
        // LP-52: Prospective Interest Rate (Numeric 5, 3.2)
        Field::new("prospective_rate", DataType::Float64, true),
        // LP-53: Liquidation Balance (Numeric 16, 13.2)
        Field::new("liquidation_balance", DataType::Float64, true),
        // LP-54: Number of Months Forbearance (Character 2)
        Field::new("forbearance_months", DataType::Utf8, true),
        // LP-55: COVID Flag — Y/N/U (Character 1)
        Field::new("covid_flag", DataType::Utf8, true),
        // LP-56: Forbearance Begin Date (Numeric 8, CCYYMMDD)
        Field::new("forbearance_begin_date", DataType::Utf8, true),
        // LP-57: Forbearance End Date (Numeric 8, CCYYMMDD)
        Field::new("forbearance_end_date", DataType::Utf8, true),
        // LP-58: Forbearance Exit Code (Character 1)
        Field::new("forbearance_exit_code", DataType::Utf8, true),
        // LP-59: Months Delinquent History — 48-char string
        Field::new("delinquency_history", DataType::Utf8, true),
        // Trailing empty field from pipe at end of line
        Field::new("trailing_pad", DataType::Utf8, true),
    ]))
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Convert year+quarter to YYYYMM for file naming
    let (yyyymm, filename) = quarter_to_yyyymm(&cli.year, &cli.quarter)?;
    let output_path = Path::new(&cli.output);

    let txt_path = if let Some(ref local) = cli.local_file {
        Path::new(local).to_path_buf()
    } else {
        // Check cache before reading cookies
        let zip_name = filename.clone();
        let txt_name = format!("LoanPerf_{}.txt", yyyymm);
        let txt_cached = Path::new(&txt_name);
        if txt_cached.exists() {
            println!("Using cached text file: {}", txt_cached.display());
            txt_cached.to_path_buf()
        } else {
            let cookies = std::fs::read_to_string(&cli.cookie_file)
                .context("Could not read cookie file — re-login at ginniemae.gov")?;
            download_and_extract(&yyyymm, &filename, &cookies, &txt_name).await?
        }
    };

    convert_to_parquet(&txt_path, output_path).await?;

    // Cache the ZIP, not the TXT (7+ GB uncompressed)
    if cli.local_file.is_none() {
        let zip_name = format!("ginnie_{}_{}.zip", cli.year, cli.quarter);
        let zip_path = Path::new(&zip_name);
        if zip_path.exists() {
            let txt_size = std::fs::metadata(&txt_path).map(|m| m.len()).unwrap_or(0);
            println!("Cleaning up extracted text file ({:.1} GB)...", txt_size as f64 / 1e9);
            std::fs::remove_file(&txt_path)?;
        }
    }

    println!("Done → {}", output_path.display());
    Ok(())
}

fn quarter_to_yyyymm(year: &str, quarter: &str) -> Result<(String, String)> {
    let end_month = match quarter.to_uppercase().as_str() {
        "Q1" => "03",
        "Q2" => "06",
        "Q3" => "09",
        "Q4" => "12",
        _ => anyhow::bail!("Invalid quarter: {}. Use Q1-Q4.", quarter),
    };
    let yyyymm = format!("{}{}", year, end_month);
    let filename = format!("LoanPerf_{}.zip", yyyymm);
    Ok((yyyymm, filename))
}

async fn download_and_extract(yyyymm: &str, filename: &str, cookies: &str, txt_name: &str) -> Result<std::path::PathBuf> {
    let zip_path = Path::new(filename);
    let txt_path = Path::new(txt_name);

    // Download ZIP
    if !zip_path.exists() {
        let url = format!(
            "https://www.ginniemae.gov/disclosure-api/api/download?dlfile=data_history_cons%2F{}",
            filename
        );
        println!("Downloading {}...", url);

        let cookie_header = cookies
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>()
            .join("; ");

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(3600))
            .build()?;

        let response = client
            .get(&url)
            .header("Cookie", cookie_header)
            .send()
            .await
            .context("Download failed — cookie may have expired")?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Download returned HTTP {} — cookie may have expired. Re-login at ginniemae.gov.",
                response.status()
            );
        }

        let total = response.content_length().unwrap_or(0);
        println!("Downloading {:.0} MB...", total as f64 / 1_000_000.0);

        let bytes = response.bytes().await?;
        std::fs::write(zip_path, &bytes)?;
        println!("Saved {} ({:.0} MB)", zip_path.display(), bytes.len() as f64 / 1_000_000.0);
    } else {
        println!("Using cached ZIP: {}", zip_path.display());
    }

    // Extract the text file
    println!("Extracting...");
    let zip_file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(zip_file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_string();
        if name.ends_with(".txt") {
            println!("Extracting {} ({:.0} MB)...", name, file.size() as f64 / 1_000_000.0);
            let mut out = std::fs::File::create(txt_path)?;
            std::io::copy(&mut file, &mut out)?;
            return Ok(txt_path.to_path_buf());
        }
    }

    anyhow::bail!("No .txt file found in ZIP archive");
}

async fn convert_to_parquet(txt_path: &Path, output_path: &Path) -> Result<()> {
    let ctx = SessionContext::new();
    let schema = build_schema();

    println!("Reading {:?}...", txt_path);

    // DataFusion CSV reader needs .csv extension and expects all lines to match schema.
    // The raw file has HH (5 fields) and TT (4 fields) mixed with LP (60 fields).
    // Filter to only LP record lines.
    let filtered = txt_path.with_extension("lp.csv");
    if !filtered.exists() {
        println!("  Filtering LP records...");
        let input = std::fs::File::open(txt_path)?;
        let output = std::fs::File::create(&filtered)?;
        let mut writer = std::io::BufWriter::new(output);
        let mut reader = std::io::BufReader::new(input);
        let mut buf = Vec::new();
        use std::io::{BufRead, Write};
        let mut line_count = 0u64;
        while reader.read_until(b'\n', &mut buf)? > 0 {
            if buf.starts_with(b"LP|") {
                writer.write_all(&buf)?;
                line_count += 1;
            }
            buf.clear();
        }
        writer.flush()?;
        println!("  Filtered {} LP records to {:?}", line_count, filtered);
    }

    // Register the CSV with pipe delimiter and no header
    ctx.register_csv(
        "loans",
        filtered.to_str().unwrap(),
        CsvReadOptions::new()
            .schema(&schema)
            .delimiter(b'|')
            .has_header(false),
    )
    .await?;

    // Count rows
    let count_df = ctx.sql("SELECT count(*) as cnt FROM loans").await?;
    let batches = count_df.collect().await?;
    let row_count: i64 = batches[0]
        .column(0)
        .as_any()
        .downcast_ref::<datafusion::arrow::array::Int64Array>()
        .unwrap()
        .value(0);
    println!("  {} rows", row_count);

    // Write Parquet
    println!("Writing Parquet to {}...", output_path.display());
    let df = ctx.sql("SELECT * FROM loans").await?;

    df.write_parquet(
        output_path.to_str().unwrap(),
        DataFrameWriteOptions::default(),
        None,
    )
    .await?;

    let parquet_size = std::fs::metadata(output_path).map(|m| m.len()).unwrap_or(0);
    println!("  Parquet: {:.1} MB", parquet_size as f64 / 1_000_000.0);

    // Clean up filtered temp file
    let _ = std::fs::remove_file(&filtered);

    Ok(())
}
