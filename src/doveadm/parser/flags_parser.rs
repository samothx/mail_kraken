use crate::doveadm::params::ImapField;
use crate::doveadm::parser::{FetchFieldRes, Parser};
use crate::doveadm::Reader;
use anyhow::{anyhow, Context, Result};
use regex::Regex;

pub struct FlagsParser {
    first_line_re: Regex,
}

impl FlagsParser {
    pub fn new() -> Result<FlagsParser> {
        let re_str = format!(r"^{}:\s+(.*)$", ImapField::Flags.to_string());
        Ok(FlagsParser {
            first_line_re: Regex::new(re_str.as_str())
                .with_context(|| format!("failed to create regex from '{}'", re_str))?,
        })
    }
}

impl Parser for FlagsParser {
    fn get_first_line_re(&self) -> &Regex {
        &self.first_line_re
    }

    fn parse_first_field(
        &self,
        reader: &mut Reader,
        _next_re: Option<&Regex>,
    ) -> Result<Option<FetchFieldRes>> {
        // this is a one-liner, so next_re is not needed
        if let Some(line) = reader.next_line()? {
            if let Some(captures) = self.first_line_re.captures(line) {
                if let Some(flags) = captures.get(1) {
                    Ok(Some(FetchFieldRes::Flags(
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
                Err(anyhow!("no match for Flags parser"))
            }
        } else {
            Ok(None)
        }
    }
}
