use datafusion::arrow::array::{Float64Array, Int32Array, StringArray};
use datafusion::arrow::datatypes::{DataType, Field, Schema};
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::parquet::arrow::ArrowWriter;
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("loan_id", DataType::Utf8, false),
        Field::new("period", DataType::Utf8, false),
        Field::new("state", DataType::Utf8, true),
        Field::new("credit_score", DataType::Int32, true),
        Field::new("upb", DataType::Float64, true),
    ]));

    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(StringArray::from(vec!["loan_001", "loan_002", "loan_003", "loan_004", "loan_005"])),
            Arc::new(StringArray::from(vec!["012024", "012024", "022024", "022024", "032024"])),
            Arc::new(StringArray::from(vec!["PA", "NY", "CA", "TX", "FL"])),
            Arc::new(Int32Array::from(vec![754, 680, 720, 690, 710])),
            Arc::new(Float64Array::from(vec![450000.0, 320000.0, 580000.0, 210000.0, 390000.0])),
        ],
    )?;

    let file = std::fs::File::create("../query/public/demo.parquet")?;
    let mut writer = ArrowWriter::try_new(file, schema.clone(), None)?;
    writer.write(&batch)?;
    writer.close()?;
    println!("Wrote 5 rows to demo.parquet");
    Ok(())
}
