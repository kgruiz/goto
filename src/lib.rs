#![allow(non_snake_case)]

pub mod cli;
pub mod commands;
pub mod output;
pub mod paths;
pub mod store;

use anyhow::Result;

pub fn Run() -> Result<()> {

    let cli = cli::ParseArgs()?;

    commands::Execute(cli)
}
