use clap::Parser;
use tbcc::cli::Options;
use tbcc::driver::compile;

fn main() {
    let options = Options::parse();
    let exit_code = match compile(&options) {
        Ok(_) => 0,
        Err(_) => 1,
    };
    std::process::exit(exit_code);
}
