use anyhow::Result;
use mail_kraken::{fetch, CmdArgs};
use structopt::StructOpt;

fn main() -> Result<()> {
    Ok(fetch(CmdArgs::from_args())?)
}
