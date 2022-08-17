use anyhow::Result;
use mail_kraken::{run, CmdArgs};
use structopt::StructOpt;

#[tokio::main]
async fn main() -> Result<()> {
    run(CmdArgs::from_args()).await
}
