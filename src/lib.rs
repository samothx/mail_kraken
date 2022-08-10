use anyhow::{anyhow, Result};
use log::info;
use mod_logger::Logger;
use nix::unistd::getuid;

mod doveadm;
use crate::doveadm::{DoveadmFetch, FetchParams, SearchParam};
pub use doveadm::CmdArgs;

pub fn fetch(cmd_args: CmdArgs) -> Result<()> {
    Logger::set_default_level(cmd_args.log_level);
    Logger::set_color(true);
    Logger::set_brief_info(true);

    if ! getuid().is_root() {
        return Err(anyhow!("please run this command as root"));
    }

    // TODO: set userid to nobody
    //

    let mut fetch_params = FetchParams::new(cmd_args.user);

    fetch_params
        .add_search_param(SearchParam::Mailbox("INBOX".to_owned()))
        .add_search_param(SearchParam::Seen);

    cmd_args.fields.iter().for_each(|field| {
        let _ = fetch_params.add_field(field.clone());
    });

    info!("fetch: calling doveadm with parameters {:?}", fetch_params);
    let mut doveadm = DoveadmFetch::new(fetch_params)?;
    while let Some(record) = doveadm.parse_record()? {
        info!("fetch: Got: \n {:?}", record);
    }
    todo!()
}


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
