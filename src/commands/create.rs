use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[clap(arg_required_else_help = true)]
pub struct Args {
    #[arg(required = true)]
    path: PathBuf,

    #[arg(short, long, required = true)]
    version: f32,
}

pub async fn execute(args: Args) -> miette::Result<()> {
    let _ = args;
    Ok(())
}
