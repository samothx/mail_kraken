use anyhow::Result;
use mod_logger::Logger;

mod doveadm;
use crate::doveadm::{DoveadmFetch, FetchParams, SearchParam};
pub use doveadm::CmdArgs;

pub fn fetch(cmd_args: CmdArgs) -> Result<()> {
    Logger::set_default_level(cmd_args.log_level);
    Logger::set_color(true);
    Logger::set_brief_info(true);

    let mut fetch_params = FetchParams::new(cmd_args.user);

    fetch_params
        .add_search_param(SearchParam::Mailbox("INBOX".to_owned()))
        .add_search_param(SearchParam::Seen);

    cmd_args.fields.iter().for_each(|field| {
        let _ = fetch_params.add_field(field.clone());
    });

    let mut doveadm = DoveadmFetch::new(fetch_params)?;
    while let Ok(record) = doveadm.parse_record() {
        eprintln!("Got: \n {:?}", record);
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
