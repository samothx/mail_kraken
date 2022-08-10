use crate::doveadm::params::ImapField;
use crate::doveadm::parser::{FetchFieldRes, Parser, SingleLineType, LINE_FEED};
use crate::doveadm::Reader;
use anyhow::{anyhow, Context, Result};
use log::debug;
use regex::Regex;

pub struct SingleLineParser {
    field_type: ImapField,
    first_line_re: Regex,
    is_list: bool,
}

impl SingleLineParser {
    pub fn new(field_type: &ImapField, is_list: bool) -> Result<SingleLineParser> {
        let re_str = format!(r"^{}:\s+(.*)$", field_type.to_string());
        debug!("FlagsParser::new: first_line_re: {:?}", re_str);
        Ok(SingleLineParser {
            field_type: field_type.clone(),
            first_line_re: Regex::new(re_str.as_str())
                .with_context(|| format!("failed to create regex from '{}'", re_str))?,
            is_list,
        })
    }
}

impl Parser for SingleLineParser {
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
            let line = line.trim_end_matches(LINE_FEED);
            if let Some(captures) = self.first_line_re.captures(line) {
                if let Some(flags) = captures.get(1) {
                    debug!(
                        "FlagsParser::parse_first_field: got payload: {:?}",
                        flags.as_str()
                    );
                    if self.is_list {
                        Ok(Some(FetchFieldRes::SingLine((
                            self.field_type.clone(),
                            SingleLineType::ListType(
                                flags
                                    .as_str()
                                    .split_whitespace()
                                    .map(|part| part.to_owned())
                                    .collect(),
                            ),
                        ))))
                    } else {
                        Ok(Some(FetchFieldRes::SingLine((
                            self.field_type.clone(),
                            SingleLineType::StringType(flags.as_str().to_owned()),
                        ))))
                    }
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
