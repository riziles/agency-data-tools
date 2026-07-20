use datafusion::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = SessionContext::new();
    ctx.register_parquet("loans", "../test-data/2024Q1.parquet", Default::default()).await?;
    let subset = ctx.sql("SELECT * FROM loans LIMIT 1000").await?;
    let count = subset.clone().count().await?;
    subset.write_parquet(
        "../query/public/loans_subset.parquet",
        datafusion::dataframe::DataFrameWriteOptions::new(),
        None,
    ).await?;
    println!("Wrote {} rows to loans_subset.parquet", count);
    Ok(())
}
