use crate::doveadm::params::ImapField;
use crate::doveadm::parser::{FetchFieldRes, Parser};
use crate::doveadm::Reader;
use anyhow::{anyhow, Context, Result};
use regex::Regex;
use log::debug;

pub struct FlagsParser {
    first_line_re: Regex,
}

impl FlagsParser {
    pub fn new() -> Result<FlagsParser> {
        let re_str = format!(r"^{}:\s+(.*)$", ImapField::Flags.to_string());
	debug!("FlagsParser::new: first_line_re: {:?}", re_str);
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
        next_re: Option<&Regex>,
    ) -> Result<Option<FetchFieldRes>> {
        // this is a one-liner, so next_re is not needed
        if let Some(line) = reader.next_line()? {
	    let line = line.trim_end_matches('\n');
            if let Some(captures) = self.first_line_re.captures(line) {
                if let Some(flags) = captures.get(1) {
		    debug!("FlagsParser::parse_first_field: got payload: {:?}", flags.as_str());
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
               let next_field_re = if let Some(next_re) = next_re {
                        next_re
                    } else {
                        &self.first_line_re
                    };
                if line.ends_with(FORM_FEED) {
                    return Ok(None)
                }|| next_field_re.is_match(line) {
                    reader.unconsume();

                Err(anyhow!("FlagsParser::parse_first_field: no match for Flags parser on {:?}", line))
            }
        } else {
            Ok(None)
        }
    }
}
