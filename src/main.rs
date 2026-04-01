use clap::Parser;
use tbcc::cli::Options;
use tbcc::driver;

fn main() {
    let options = Options::parse();
    let exit_code = match driver::compile(&options) {
        Ok(_) => 0,
        Err(error    ) => {
            eprintln!("{}", error);
            1
        },
    };
    std::process::exit(exit_code);
}
