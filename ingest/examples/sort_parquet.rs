use datafusion::prelude::*;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = SessionContext::new();
    
    // Register parquet file
    let file_path = Path::new("../test-data/2024Q1.parquet").canonicalize()?;
    let uri = format!("file://{}", file_path.display());
    ctx.register_parquet("loans", &uri, Default::default()).await?;
    
    println!("Reading + sorting by property_state...");
    let df = ctx.sql("SELECT * FROM loans ORDER BY property_state").await?;
    
    println!("Writing sorted parquet...");
    use datafusion::dataframe::DataFrameWriteOptions;
    df.write_parquet("../test-data/2024Q1-sorted.parquet", DataFrameWriteOptions::new(), None).await?;
    
    println!("Done!");
    Ok(())
}
