use anyhow::Result;
use std::path::PathBuf;
use telemetry_core::schema;

fn main() -> Result<()> {
    let out_dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("telemetry/schemas"));
    schema::generate_schemas(&out_dir)?;
    println!("Schemas written to {}", out_dir.display());
    Ok(())
}
