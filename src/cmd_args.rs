use mod_logger::Level;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "mail_kraken", about = "analyse - analyse mailbox")]
pub struct ServeArgs {
    #[structopt(
        short,
        long,
        value_name = "LOGLEVEL",
        help = "Log Level, one of (error, warn, info, debug, trace)",
        default_value = "info"
    )]
    pub log_level: Level,
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

#[derive(StructOpt)]
#[structopt(name = "import", about = "import mailbox to mail_kraken database")]
pub struct ImportArgs {
    #[structopt(
        short,
        long,
        value_name = "LOGLEVEL",
        help = "Log Level, one of (error, warn, info, debug, trace)",
        default_value = "info"
    )]
    pub log_level: Level,
    #[structopt(
        short,
        long,
        value_name = "USER",
        help = "The email address/user to import"
    )]
    pub user: String,

    #[structopt(
        short,
        long,
        value_name = "COPY_TO",
        help = "Debug copy of doveadm fetch output",
        parse(from_os_str)
    )]
    pub copy_to: Option<PathBuf>,
}
