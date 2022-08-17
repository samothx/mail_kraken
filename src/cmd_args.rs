use mod_logger::Level;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "mail_kraken", about = "analyse - analyse mailbox")]
pub struct CmdArgs {
    #[structopt(subcommand)]
    pub cmd: Command,
    #[structopt(
        short,
        long,
        value_name = "LOGLEVEL",
        help = "Log Level, one of (error, warn, info, debug, trace)",
        default_value = "info"
    )]
    pub log_level: Level,
}

#[derive(Debug, StructOpt)]
pub enum Command {
    Serve(ServeCmd),
    // TODO: Install - install program, create user & group, copy templates & html-files,
    // optionally create local database - or cargo make install
}

#[derive(Debug, StructOpt)]
pub struct ServeCmd {
    #[structopt(
        short,
        long,
        value_name = "BIND_TO",
        help = "Address / port to serve content on",
        default_value = "127.0.0.1:8080"
    )]
    pub bind_to: String,
    #[structopt(
        short,
        long,
        value_name = "INIT_PASSWD",
        help = "Initial password for admin",
        default_value = "5ecr3t"
    )]
    pub init_passwd: String,
}
