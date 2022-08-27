use crate::doveadm::fetch::parser::{FetchFieldRes, Parser, LINE_FEED};
use crate::doveadm::fetch::Reader;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use log::debug;
use regex::Regex;

macro_rules! string_parser {
    ($name:ident,$tag:expr,$res:expr) => {
        pub struct $name {
            first_line_re: Regex,
        }

        impl $name {
            pub fn new() -> Result<$name> {
                let re_str = format!(r"^{}:\s+(.*)$", $tag);
                debug!("new: [{}]->str first_line_re: {:?}", $tag, re_str);
                Ok($name {
                    first_line_re: Regex::new(re_str.as_str()).with_context(|| {
                        format!(
                            "new: [{}]->str failed to create regex from '{}'",
                            $tag, re_str
                        )
                    })?,
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
                            debug!(
                                "parse_first_field: [{}]->str got payload: {:?}",
                                $tag,
                                uid.as_str()
                            );
                            Ok(Some($res(uid.as_str().to_owned())))
                        } else {
                            Err(anyhow!(
                                "parse_first_field: [{}]->str parser matched but no caption",
                                $tag
                            )) //
                        }
                    } else {
                        Err(anyhow!(
                            "parse_first_field: [{}]->str no match for parser on {:?}",
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

macro_rules! string_list_parser {
    ($name:ident,$tag:expr,$res:expr) => {
        pub struct $name {
            first_line_re: Regex,
        }

        impl $name {
            pub fn new() -> Result<$name> {
                let re_str = format!(r"^{}:\s+(.*)$", $tag);
                debug!("new: [{}]->sl first_line_re: {:?}", $tag, re_str);
                Ok($name {
                    first_line_re: Regex::new(re_str.as_str()).with_context(|| {
                        format!(
                            "new: [{}]->sl failed to create regex from '{}'",
                            $tag, re_str
                        )
                    })?,
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
                        if let Some(flags) = captures.get(1) {
                            debug!(
                                "parse_first_field: [{}]->sl got payload: {:?}",
                                $tag,
                                flags.as_str()
                            );
                            Ok(Some($res(
                                flags
                                    .as_str()
                                    .split_whitespace()
                                    .map(|part| part.to_owned())
                                    .collect(),
                            )))
                        } else {
                            Err(anyhow!(
                                "parse_first_field: [{}]->sl parser matched but no caption",
                                $tag
                            )) //
                        }
                    } else {
                        Err(anyhow!(
                            "parse_first_field: [{}]->sl no match for Flags parser on {:?}",
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

string_parser!(UidParser, "uid", FetchFieldRes::Uid);
string_parser!(GuidParser, "guid", FetchFieldRes::Guid);
string_parser!(MailboxParser, "mailbox", FetchFieldRes::Mailbox);

string_list_parser!(FlagsParser, "flags", FetchFieldRes::Flags);
