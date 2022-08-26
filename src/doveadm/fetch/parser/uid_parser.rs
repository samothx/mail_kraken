use crate::doveadm::fetch::params::ImapField;
use crate::doveadm::fetch::parser::{FetchFieldRes, Parser, SingleLineType, LINE_FEED};
use crate::doveadm::fetch::Reader;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use log::debug;
use regex::Regex;

pub struct UidParser {
    first_line_re: Regex,
}

impl UidParser {
    pub fn new() -> Result<UidParser> {
        let re_str = r"^uid:\s+(.*)$";
        debug!("new: first_line_re: {:?}", re_str);
        Ok(UidParser {
            first_line_re: Regex::new(re_str)
                .with_context(|| format!("failed to create regex from '{}'", re_str))?,
        })
    }
}

#[async_trait]
impl Parser for UidParser {
    fn get_first_line_re(&self) -> &Regex {
        &self.first_line_re
    }

    async fn parse_first_field(
        &self,
        reader: &mut Reader,
        _next_re: Option<&Regex>,
    ) -> Result<Option<FetchFieldRes>> {
        // this is a one-liner, so next_re is not needed
        if let Some(line) = reader.next_line().await? {
            let line = line.trim_end_matches(LINE_FEED);
            if let Some(captures) = self.first_line_re.captures(line) {
                if let Some(uid) = captures.get(1) {
                    debug!("parse_first_field: got payload: {:?}", uid.as_str());
                    Ok(Some(FetchFieldRes::Uid(uid.as_str().to_owned())))
                } else {
                    Err(anyhow!("Uid parser matched but no caption")) //
                }
            } else {
                Err(anyhow!(
                    "parse_first_field: no match for Uid parser on {:?}",
                    line
                ))
            }
        } else {
            Ok(None)
        }
    }
}

macro_rules! string_parser {
    ($name:ident,$tag:expr) => {
        pub struct $name {
            first_line_re: Regex,
        }

        impl $name {
            pub fn new() -> Result<$name> {
                let re_str = format!(r"^{}:\s+(.*)$", $tag);
                debug!("new: first_line_re: {:?}", re_str);
                Ok($name {
                    first_line_re: Regex::new(re_str.as_str())
                        .with_context(|| format!("failed to create regex from '{}'", re_str))?,
                })
            }
        }

        #[async_trait]
        impl Parser for $name {
            fn get_first_line_re(&self) -> &Regex {
                &self.first_line_re
            }

            async fn parse_first_field(
                &self,
                reader: &mut Reader,
                _next_re: Option<&Regex>,
            ) -> Result<Option<FetchFieldRes>> {
                // this is a one-liner, so next_re is not needed
                if let Some(line) = reader.next_line().await? {
                    let line = line.trim_end_matches(LINE_FEED);
                    if let Some(captures) = self.first_line_re.captures(line) {
                        if let Some(uid) = captures.get(1) {
                            debug!("parse_first_field: got payload: {:?}", uid.as_str());
                            Ok(Some(FetchFieldRes::Uid(uid.as_str().to_owned())))
                        } else {
                            Err(anyhow!("{} parser matched but no caption", $tag)) //
                        }
                    } else {
                        Err(anyhow!(
                            "parse_first_field: no match for {} parser on {:?}",
                            $tag,
                            line
                        ))
                    }
                } else {
                    Ok(None)
                }
            }
        }
    };
}

string_parser!(GuidParser, "guid");
