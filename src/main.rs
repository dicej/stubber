use std::io::{Read, Write};

use anyhow::Result;
use clap::Parser;
use stubber::{replace, Options};

fn main() -> Result<()> {
    let options = Options::parse();

    let mut wasm = Vec::new();
    std::io::stdin().read_to_end(&mut wasm)?;

    let wasm = replace(options, wasm)?;
    std::io::stdout().write_all(&wasm)?;

    return Ok(());
}
