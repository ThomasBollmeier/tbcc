#[derive(clap::Parser, Debug, Clone)]
#[command(author, version)]
#[command(
    help_template = "{name} - {about} [version: {version}, author: {author}]\n\n{usage-heading} {usage}\n\n{all-args}"
)]
#[command(about="A compiler for a simplified C", long_about = None)]
#[command(group(
    clap::ArgGroup::new("compile_step")
        .args(["lex", "parse", "validate", "tacky", "codegen", "dont_assemble"])
        .multiple(false)
))]
pub struct Options {
    pub source: String,
    #[arg(long, help = "Stop after lexing")]
    pub lex: bool,
    #[arg(long, help = "Stop after parsing")]
    pub parse: bool,
    #[arg(long, help = "Stop after validation")]
    pub validate: bool,
    #[arg(long, help = "Stop after tacky generation")]
    pub tacky: bool,
    #[arg(long, help = "Stop before code generation")]
    pub codegen: bool,
    #[arg(short = 'S', help = "do not assemble and link")]
    pub dont_assemble: bool,
}
