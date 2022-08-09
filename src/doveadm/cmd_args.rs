use crate::doveadm::ImapField;
use mod_logger::Level;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "analyse", about = "analyse - analyse mailbox")]
pub struct CmdArgs {
    #[structopt(short, long, value_name = "USER", help = "fully email of a valid user")]
    pub user: String,
    #[structopt(
        short,
        long,
        value_name = "LOGLEVEL",
        help = "Log Level, one of (error, warn, info, debug, trace)",
        default_value = "info"
    )]
    pub log_level: Level,

    #[structopt(short = "f", long = "fields")]
    pub fields: Vec<ImapField>,
}
