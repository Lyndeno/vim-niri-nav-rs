use clap::{CommandFactory, ValueEnum};
use clap_complete::{generate_to, Shell};
use std::env;
use std::io::Error;

#[path = "src/args.rs"]
mod args;
use crate::args::Args;

fn main() -> Result<(), Error> {
    let Some(outdir) = env::var_os("OUT_DIR") else {
        return Ok(());
    };

    let mut cmd = <Args as CommandFactory>::command();
    for &shell in Shell::value_variants() {
        generate_to(shell, &mut cmd, "vim-niri-nav", outdir.clone())?;
    }

    Ok(())
}
