use clap::Parser;

#[derive(Parser, Debug)]
#[clap(arg_required_else_help = true)]
pub struct Args;

pub fn execute(args: Args) -> miette::Result<()> {
    let _ = args;
    Ok(())
}
