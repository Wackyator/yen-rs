use clap::Parser;

use crate::github::list_pythons;

#[derive(Parser, Debug)]
pub struct Args;

pub async fn execute(args: Args) -> miette::Result<()> {
    let _ = args;
    let _ = list_pythons().await?;
    Ok(())
}
