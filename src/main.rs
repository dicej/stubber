use anyhow::Result;
use clap::Parser;
use stubber::{replace, Options};

fn main() -> Result<()> {
    let options = Options::parse();
    replace(options)?;

    return Ok(());
}
