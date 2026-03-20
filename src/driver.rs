use anyhow::{anyhow, Result};
use std::path::Path;
use std::process::Command;
use crate::assembly_ast::AssemblyCreator;
use crate::cli::Options;
use crate::lexer::Lexer;
use crate::parser::Parser;

pub fn compile(options: &Options) -> Result<()> {
    let preprocessed_file = preprocess(&options.source)?;
    let code = std::fs::read_to_string(&preprocessed_file)?;

    let lexer = Lexer::new();
    let tokens = lexer.scan_tokens(&code)?;

    if options.lex {
        return Ok(());
    }

    let parser = Parser::new();
    let program = parser.parse(tokens)?;

    if options.parse {
        return Ok(());
    }

    let mut assembly_creator = AssemblyCreator::new();
    let _asm_program = assembly_creator.create_assembly_program(&program)?;

    if options.codegen {
        return Ok(());
    }

    // generate code ...

    if !options.dont_assemble {
        // assemble and link...
    }

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
