use datafusion::prelude::*;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = std::env::args().nth(1).unwrap_or_else(|| "../test-data/2024Q1.parquet".into());
    let output = std::env::args().nth(2).unwrap_or_else(|| "../test-data/2024Q1-sorted.parquet".into());
    let sort_col = std::env::args().nth(3).unwrap_or_else(|| "property_state".into());

    let ctx = SessionContext::new();
    let p = Path::new(&input).canonicalize()?;
    let uri = format!("file://{}", p.display());
    
    println!("Registering: {uri}");
    ctx.register_parquet("loans", &uri, Default::default()).await?;

    println!("Sorting by: {sort_col}");
    let df = ctx.sql(&format!("SELECT * FROM loans ORDER BY {sort_col}")).await?;

    println!("Writing: {output}");
    use datafusion::dataframe::DataFrameWriteOptions;
    df.write_parquet(&output, DataFrameWriteOptions::new(), None).await?;
    println!("Done!");
    Ok(())
}
