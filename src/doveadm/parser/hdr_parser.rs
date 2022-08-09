use crate::doveadm::params::ImapField;
use crate::doveadm::parser::FetchFieldRes::Hdr;
use crate::doveadm::parser::{FetchFieldRes, Parser};
use crate::doveadm::{Reader, FORM_FEED, LINE_FEED};
use anyhow::{anyhow, Context, Result};
use regex::Regex;
use log::debug;

pub struct HdrParser {
    first_line_re: Regex,
    subseq_line_re: Regex,
}
impl HdrParser {
    pub fn new() -> Result<HdrParser> {
        let re_str = format!(r"^{}:$", ImapField::Hdr.to_string());
        debug!("first_line_re: {:?}", re_str);
        let subseq_re_str = r"^([\S^:]+):\s(.*)$";
        Ok(HdrParser {
            first_line_re: Regex::new(re_str.as_str())
                .with_context(|| format!("failed to create regex from '{}'", re_str))?,
            subseq_line_re: Regex::new(subseq_re_str)
                .with_context(|| format!("failed to create regex from '{}'", subseq_re_str))?,
        })
    }
}

impl Parser for HdrParser {
    fn get_first_line_re(&self) -> &Regex {
        &self.first_line_re
    }

    fn parse_first_field(
        &self,
        reader: &mut Reader,
        next_re: Option<&Regex>,
    ) -> Result<Option<FetchFieldRes>> {
        if let Some(line) = reader.next_line()? {
            if self.first_line_re.is_match(line) {
                // hdr: found
                let next_field_re = if let Some(next_re) = next_re {
                    next_re
                } else {
                    &self.first_line_re
                };
                let mut res: Vec<(String, String)> = Vec::new();
                while let Some(line) = reader.next_line()? {
                    let line = line.trim_end_matches(LINE_FEED);
                    if line.ends_with(FORM_FEED) || next_field_re.is_match(line) {
                        reader.unconsume();
                        if res.len() > 0 {
                            return Ok(Some(FetchFieldRes::Hdr(res)));
                        } else {
                            // TODO: or accept an empty field res ?
                            return Err(anyhow!(
                                "HdrParser::parse_first_field: unexpected empty field"
                            ));
                        }
                    } else {
                        if let Some(captures) = self.subseq_line_re.captures(line) {
                            res.push((
                                captures
                                    .get(1)
                                    .expect(
                                        format!("HdrParser::parse_first_field: unexpected empty Hdr name in line '{}'", line)
                                            .as_str(),
                                    )
                                    .as_str()
                                    .to_owned(),
                                captures
                                    .get(2)
                                    .expect(
                                        format!("HdrParser::parse_first_field: unexpected empty Hdr value in line '{}'", line)
                                            .as_str(),
                                    )
                                    .as_str()
                                    .to_owned(),
                            ));
                        } else {
                            if let Some(last_res) = res.last_mut() {
                                last_res.1.push('\n');
                                last_res.1.push_str(line);
                            } else {
                                return Err(anyhow!(
                                    "HdrParser::parse_first_field: hdr regex failed to match in line '{}'",
                                    line
                                ));
                            }
                        }
                    }
                }
            } else {
                debug!("first line re did not match {:?}", line);
                return Err(anyhow!(
                    "HdrParser::parse_first_field: Hdr parser failed to match first line"
                ));
            }
            // TODO: really ? or rather return what we have got
            Err(anyhow!("unexpected EOI in field"))
        } else {
            Ok(None)
        }
    }
}
