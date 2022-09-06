use anyhow::Result;
use mail_kraken::{serve, ServeArgs};
use structopt::StructOpt;

#[tokio::main]
async fn main() -> Result<()> {
    serve(ServeArgs::from_args()).await
}
