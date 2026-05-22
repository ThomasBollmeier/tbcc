use crate::assembly;
use crate::cli::Options;
use crate::assembly::codegen::CodeGenerator;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::semantic;
use crate::tacky::TackyEmitter;
use anyhow::{Result, anyhow};
use std::path::Path;
use std::process::Command;

pub fn compile(options: &Options) -> Result<()> {
    validate_options(options)?;

    let mut assembly_files = vec![];

    for source_file in &options.sources {
        if let Some(assembly_file) = compile_file(source_file, options)? {
            assembly_files.push(assembly_file);
        }
    }

    if assembly_files.is_empty() {
        return Ok(());
    }

    if !options.dont_assemble {
        create_output_file(&assembly_files, options)?;
        for assembly_file in &assembly_files {
            remove_file(assembly_file)?;
        }
    }

    Ok(())
}

fn validate_options(options: &Options) -> Result<()> {
    if options.dont_link && options.sources.len() > 1 {
        return Err(anyhow!(
            r#"Cannot create object file when multiple assembly files are generated.
Please use -S to stop before assembling or remove the -c flag to link the files together."#
        ));
    }

    Ok(())
}

fn compile_file(source_file: &str, options: &Options) -> Result<Option<String>> {
    let preprocessed_file = create_preprocessed_file(&source_file)?;
    let code = std::fs::read_to_string(&preprocessed_file)?;

    let lexer = Lexer::new();
    let tokens = lexer.scan_tokens(&code)?;

    remove_file(&preprocessed_file)?;

    if options.lex {
        return Ok(None);
    }

    let parser = Parser::new();
    let mut program = parser.parse(tokens)?;

    if options.parse {
        return Ok(None);
    }

    let var_name_generator = semantic::make_var_name_generator();
    let label_name_generator = semantic::make_label_name_generator();

    semantic::validate(&var_name_generator, &label_name_generator, &mut program)?;

    if options.validate {
        return Ok(None);
    }

    let tmp_var_name_generator = semantic::make_temp_var_name_generator();
    let mut tacky_emitter = TackyEmitter::new(label_name_generator, tmp_var_name_generator);
    let tacky_program = tacky_emitter.emit_program(&program)?;

    if options.tacky {
        return Ok(None);
    }

    let asm_program = assembly::create_program(&tacky_program)?;

    if options.codegen {
        return Ok(None);
    }

    let assembly_code = CodeGenerator::new().generate_assembly(&asm_program);
    let assembly_file = create_assembly_file(&source_file, &assembly_code)?;

    Ok(Some(assembly_file))
}

fn create_output_file(assembly_files: &Vec<String>, options: &Options) -> Result<()> {
    let new_extension = if !options.dont_link { "" } else { "o" };

    let main_file = assembly_files.get(0).unwrap();
    let output_file = create_file_name_with_new_extension(main_file, new_extension)?;

    let status = {
        let mut cmd = Command::new("gcc");
        for assembly_file in assembly_files {
            cmd.arg(assembly_file);
        }
        if options.dont_link {
            cmd.arg("-c");
        }
        let status = cmd.arg("-o").arg(&output_file).status()?;
        status
    };

    if !status.success() {
        return Err(anyhow!("gcc preprocessing failed with status: {status}"));
    }

    Ok(())
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
