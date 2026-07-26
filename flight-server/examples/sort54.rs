use datafusion::prelude::*;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = "../test-data/2024Q1.parquet";
    let output = "../test-data/2024Q1-sorted.parquet";
    
    let ctx = SessionContext::new();
    let p = Path::new(input).canonicalize()?;
    let uri = format!("file://{}", p.display());
    ctx.register_parquet("loans", &uri, Default::default()).await?;
    
    println!("Sorting by property_state...");
    let df = ctx.sql("SELECT * FROM loans ORDER BY property_state").await?;
    
    use datafusion::dataframe::DataFrameWriteOptions;
    df.write_parquet(output, DataFrameWriteOptions::new(), None).await?;
    println!("Done!");
    Ok(())
}
