use crate::doveadm::params::ImapField;
use crate::doveadm::parser::{FetchFieldRes, Parser, LINE_FEED};
use crate::doveadm::Reader;
use anyhow::{anyhow, Context, Result};
use log::{debug, warn};
use regex::Regex;
use std::collections::HashMap;

pub struct HdrParser {
    first_line_re: Regex,
    subseq_line_re: Regex,
}
impl HdrParser {
    pub fn new() -> Result<HdrParser> {
        let re_str = format!(r"^{}:$", ImapField::Hdr.to_string());
        let subseq_re_str = r"^(([^:]+):\s(.*)|\s*(\S.*))$";
        debug!("first_line_re:  {:?}", re_str);
        debug!("subseq_line_re: {:?}", subseq_re_str);
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
        _next_re: Option<&Regex>,
    ) -> Result<Option<FetchFieldRes>> {
        if let Some(line) = reader.next_line()? {
            let line = line.trim_end_matches(LINE_FEED);
            if self.first_line_re.is_match(line) {
                let mut res: HashMap<String, String> = HashMap::new();
                let mut last_key: Option<String> = None;
                while let Some(line) = reader.next_line()? {
                    let line = line.trim_end_matches(LINE_FEED);
                    if let Some(captures) = self.subseq_line_re.captures(line) {
                        if let Some(no_tag) = captures.get(4) {
                            let add_val = no_tag.as_str();
                            if !add_val.is_empty() {
                                if let Some(key) = last_key.as_ref() {
                                    let value = res.get_mut(key).expect("unexpected key not found");
                                    value.push('\n');
                                    value.push_str(add_val);
                                } else {
                                    warn!("no recent key found fo tagless value");
                                }
                            } else {
                                return Ok(Some(FetchFieldRes::Hdr(res)));
                            }
                        } else {
                            let key = captures
                                    .get(2)
                                    .expect(
                                        format!("HdrParser::parse_first_field: unexpected empty Hdr name in line '{}'", line)
                                            .as_str(),
                                    )
                                    .as_str()
                                    .to_owned();

                            res.insert(
                                    key.clone(),
                                    captures
                                        .get(3)
                                        .expect(
                                            format!("HdrParser::parse_first_field: unexpected empty Hdr value in line '{}'", line)
                                                .as_str(),
                                        )
                                        .as_str()
                                        .to_owned(),
                                ).map_or((), |_| warn!("duplicate key found: '{}'", key));
                            last_key = Some(key);
                        }
                    } else {
                        return Err(anyhow!("hdr regex failed to match in line '{}'", line));
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
