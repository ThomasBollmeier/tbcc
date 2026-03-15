use anyhow::{anyhow, Result};
use std::path::Path;
use std::process::Command;
use crate::cli::Options;

pub fn compile(options: &Options) -> Result<()> {
    let _preprocessed_file = preprocess(&options.source)?;



    Ok(())
}


fn preprocess(source_file: &str) -> Result<String> {
    let source_path = Path::new(source_file);
    let preprocessed_file = source_path
        .with_extension("i");
    let preprocessed_file = preprocessed_file
        .to_str()
        .ok_or_else(|| anyhow!("Failed to convert preprocessed file path to string"))?;

    let status = Command::new("gcc")
        .arg("-E")
        .arg("-P")
        .arg(source_file)
        .arg("-o")
        .arg(preprocessed_file)
        .status()?;

    if !status.success() {
        return Err(anyhow!("gcc preprocessing failed with status: {status}"));
    }

    Ok(preprocessed_file.to_string())
}
