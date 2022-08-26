use crate::doveadm::fetch::params::ImapField;
use crate::doveadm::fetch::parser::{FetchFieldRes, Parser, SingleLineType, LINE_FEED};
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
                            Ok(Some($res(uid.as_str().to_owned())))
                        } else {
                            Err(anyhow!(
                                "parse_first_field: {} parser matched but no caption",
                                $tag
                            )) //
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

macro_rules! string_list_parser {
    ($name:ident,$tag:expr) => {
        pub struct $name {
            first_line_re: Regex,
        }

        impl $name {
            pub fn new() -> Result<$name> {
                let re_str = format!(r"^{}:\s+(.*)$", $tag);
                debug!("new: {} first_line_re: {:?}", $tag, re_str);
                Ok(SingleLineParser {
                    field_type: field_type.clone(),
                    first_line_re: Regex::new(re_str.as_str()).with_context(|| {
                        format!("new: {} failed to create regex from '{}'", $tag, re_str)
                    })?,
                    is_list,
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
                            debug!("parse_first_field: got payload: {:?}", flags.as_str());
                            Ok(Some(FetchFieldRes::$res(
                                flags
                                    .as_str()
                                    .split_whitespace()
                                    .map(|part| part.to_owned())
                                    .collect(),
                            )))
                        } else {
                            Err(anyhow!("Flags parser matched but no caption")) //
                        }
                    } else {
                        Err(anyhow!(
                            "FlagsParser::parse_first_field: no match for Flags parser on {:?}",
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
