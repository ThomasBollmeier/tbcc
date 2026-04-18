use crate::assembly;
use crate::cli::Options;
use crate::codegen::CodeGenerator;
use crate::lexer::Lexer;
use crate::parser::Parser;
use anyhow::{Result, anyhow};
use std::path::Path;
use std::process::Command;
use crate::semantic;
use crate::semantic::{LabelResolver, VariableResolver};
use crate::tacky::TackyEmitter;

pub fn compile(options: &Options) -> Result<()> {
    let preprocessed_file = create_preprocessed_file(&options.source)?;
    let code = std::fs::read_to_string(&preprocessed_file)?;

    let lexer = Lexer::new();
    let tokens = lexer.scan_tokens(&code)?;

    remove_file(&preprocessed_file)?;

    if options.lex {
        return Ok(());
    }

    let parser = Parser::new();
    let mut program = parser.parse(tokens)?;

    if options.parse {
        return Ok(());
    }

    let var_name_generator = semantic::make_var_name_generator();
    let mut variable_resolver = VariableResolver::new(var_name_generator.clone());
    variable_resolver.resolve(&mut program)?;

    let label_name_generator = semantic::make_label_name_generator();
    let mut label_resolver = LabelResolver::new(label_name_generator.clone());
    label_resolver.resolve(&mut program)?;

    if options.validate {
        return Ok(());
    }

    let tmp_var_name_generator = semantic::make_temp_var_name_generator();

    let mut tacky_emitter = TackyEmitter::new(label_name_generator, tmp_var_name_generator);
    let tacky_program = tacky_emitter.emit_program(&program)?;

    if options.tacky {
        return Ok(());
    }

    let asm_program = assembly::create_program(&tacky_program)?;

    if options.codegen {
        return Ok(());
    }

    let assembly_code = CodeGenerator::new().generate_assembly(&asm_program);
    let assembly_file = create_assembly_file(&options.source, &assembly_code)?;

    if !options.dont_assemble {
        create_exec_file(&assembly_file)?;
        remove_file(&assembly_file)?;
    }

    Ok(())
}

fn create_exec_file(assembly_file: &str) -> Result<String> {
    let exec_file = create_file_name_with_new_extension(assembly_file, "")?;

    let status = Command::new("gcc")
        .arg(assembly_file)
        .arg("-o")
        .arg(&exec_file)
        .status()?;

    if !status.success() {
        return Err(anyhow!("gcc preprocessing failed with status: {status}"));
    }

    Ok(exec_file)
}

fn create_assembly_file(source_file: &str, assembly_code: &str) -> Result<String> {
    let assembly_file_name = create_file_name_with_new_extension(source_file, "s")?;
    std::fs::write(&assembly_file_name, assembly_code)?;
    Ok(assembly_file_name)
}

fn create_preprocessed_file(source_file: &str) -> Result<String> {
    let preprocessed_file = create_file_name_with_new_extension(source_file, "i")?;

    let status = Command::new("gcc")
        .arg("-E")
        .arg("-P")
        .arg(source_file)
        .arg("-o")
        .arg(&preprocessed_file)
        .status()?;

    if !status.success() {
        return Err(anyhow!("gcc preprocessing failed with status: {status}"));
    }

    Ok(preprocessed_file)
}

fn create_file_name_with_new_extension(source_file: &str, new_extension: &str) -> Result<String> {
    let source_path = Path::new(source_file);
    let new_path = source_path.with_extension(new_extension);
    let new_path = new_path
        .to_str()
        .ok_or_else(|| anyhow!("Failed to convert new file path to string"))?;
    Ok(new_path.to_string())
}

fn remove_file(file_name: &str) -> Result<()> {
    std::fs::remove_file(file_name)?;
    Ok(())
}
