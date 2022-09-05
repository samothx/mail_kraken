use anyhow::Result;
use mail_kraken::{import, ImportArgs};
use structopt::StructOpt;

fn main() -> Result<()> {
    import(ImportArgs::from_args())
}
