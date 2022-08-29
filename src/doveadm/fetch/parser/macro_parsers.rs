use crate::doveadm::fetch::parser::{FetchFieldRes, Parser, LINE_FEED};
use crate::doveadm::fetch::Reader;
use crate::doveadm::ImapField;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, FixedOffset, NaiveDateTime};
use log::debug;
use regex::Regex;

trait TryToRes<T> {
    fn try_to_res(self) -> Result<T>;
}

impl TryToRes<String> for &str {
    fn try_to_res(self) -> Result<String> {
        Ok(self.to_owned())
    }
}

impl TryToRes<usize> for &str {
    fn try_to_res(self) -> Result<usize> {
        Ok(self.parse()?)
    }
}

/*
impl TryToRes<NaiveDateTime> for &str {
    fn try_to_res(self) -> Result<NaiveDateTime> {
        Ok(NaiveDateTime::parse_from_str(self, "%Y-%m-%d %H:%M:%S")?)
    }
}

impl TryToRes<DateTime<FixedOffset>> for &str {
    fn try_to_res(self) -> Result<DateTime<FixedOffset>> {
        Ok(DateTime::<FixedOffset>::parse_from_str(
            self,
            "%Y-%m-%d %H:%M:%S (%z)",
        )?)
    }
}
*/

impl TryToRes<Vec<String>> for &str {
    fn try_to_res(self) -> Result<Vec<String>> {
        Ok(self
            .split_whitespace()
            .map(|part| part.to_owned())
            .collect())
    }
}

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
                        if let Some(capture) = captures.get(1) {
                            let str_val = capture.as_str();
                            debug!(
                                "parse_first_field: [{}]->str got payload: {:?}",
                                $tag, str_val
                            );
                            Ok(Some($res(str_val.try_to_res()?)))
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

string_parser!(
    UidParser,
    ImapField::Uid.to_string().as_str(),
    FetchFieldRes::Uid
);
string_parser!(
    GuidParser,
    ImapField::Guid.to_string().as_str(),
    FetchFieldRes::Guid
);
string_parser!(
    MailboxParser,
    ImapField::Mailbox.to_string().as_str(),
    FetchFieldRes::Mailbox
);
string_parser!(
    DateReceivedParser,
    ImapField::DateReceived.to_string().as_str(),
    FetchFieldRes::DateReceived
);
string_parser!(
    DateSentParser,
    ImapField::DateSent.to_string().as_str(),
    FetchFieldRes::DateSent
);
string_parser!(
    DateSavedParser,
    ImapField::DateSaved.to_string().as_str(),
    FetchFieldRes::DateSaved
);
string_parser!(
    SizePhysicalParser,
    ImapField::SizePhysical.to_string().as_str(),
    FetchFieldRes::SizePhysical
);

string_parser!(FlagsParser, "flags", FetchFieldRes::Flags);
