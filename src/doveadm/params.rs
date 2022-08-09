use anyhow::{anyhow, Error, Result};
use chrono::NaiveDate;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use std::string::ToString;
use std::time::Instant;
use strum_macros;

#[derive(Debug)]
pub struct FetchParams {
    user: String,
    fields: Vec<ImapField>,
    search: Vec<SearchParam>,
}

impl FetchParams {
    pub fn new(user: String) -> FetchParams {
        FetchParams {
            user,
            fields: Vec::new(),
            search: Vec::new(),
        }
    }

    pub fn add_field(&mut self, field: ImapField) -> &mut Self {
        self.fields.push(field);
        self
    }

    pub fn add_search_param(&mut self, param: SearchParam) -> &mut Self {
        self.search.push(param);
        self
    }

    pub fn to_args(&self) -> Result<Vec<String>> {
        let mut args = vec!["-f".to_string()];
        args.push("pager".to_string());

        args.push("fetch".to_owned());

        args.push("-u".to_string());
        args.push(self.user.clone());

        if self.fields.is_empty() {
            return Err(anyhow!("no fields in doveadm fetch params"));
        } else {
            let mut fields = String::new();
            self.fields.iter().for_each(|field| {
                fields.push_str(field.to_string().as_str());
                fields.push(' ')
            });
            args.push(fields.trim().to_owned());
        }

        if self.search.is_empty() {
            return Err(anyhow!("no search params in doveadm fetch params"));
        } else {
            for param in self.search.iter() {
                args.append(&mut param.to_params())
            }
        }

        Ok(args)
    }

    pub fn fields(&self) -> &Vec<ImapField> {
        &self.fields
    }
}

trait ToParam {
    fn to_param(&self) -> String;
}

trait ToParams {
    fn to_params(&self) -> Vec<String>;
}

#[derive(Clone, Debug, strum_macros::Display)]
pub enum ImapField {
    #[strum(serialize = "hdr")]
    Hdr,
    #[strum(serialize = "flags")]
    Flags,
    #[strum(serialize = "body")]
    Body,
    #[strum(serialize = "date.received")]
    DateReceived,
    #[strum(serialize = "date.saved")]
    DateSaved,
    #[strum(serialize = "date.sent")]
    DateSent,
    #[strum(serialize = "guid")]
    Guid,
    #[strum(serialize = "imap.body")]
    ImapBody,
    #[strum(serialize = "imap.bodystructure")]
    ImapBodystructure,
    #[strum(serialize = "imap.envelope")]
    ImapEnvelope,
    #[strum(serialize = "mailbox")]
    Mailbox,
    #[strum(serialize = "mailbox-guid")]
    MailboxGuid,
}

impl FromStr for ImapField {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "hdr" => Ok(ImapField::Hdr),
            "flags" => Ok(ImapField::Flags),
            "body" => Ok(ImapField::Body),
            "datereceived" => Ok(ImapField::DateReceived),
            "datesaved" => Ok(ImapField::DateSaved),
            "datesent" => Ok(ImapField::DateSent),
            "guid" => Ok(ImapField::Guid),
            "imap.body" => Ok(ImapField::ImapBody),
            "imap.bodystructure" | "bodystructure" => Ok(ImapField::ImapBodystructure),
            "imap.envelope" | "envelope" => Ok(ImapField::ImapEnvelope),
            "mailbox" => Ok(ImapField::Mailbox),
            "mailboxguid" | "mailbox-guid" => Ok(ImapField::MailboxGuid),
            _ => Err(anyhow!("invalid field name {}", s)),
        }
    }
}

#[derive(Debug, strum_macros::Display)]
pub enum SearchParam {
    SequenceSet(SeqSet),
    All,
    Answered,
    Bcc(String),
    Before(DateSpec),
    Body(String),
    CC(String),
    Deleted,
    Draft,
    Flagged,
    From(String),
    Header(String, Option<String>),
    Keyword(String),
    Larger(usize),
    Mailbox(String),
    #[strum(serialize = "MAILBOX-GUID")]
    MailboxGuid(String),
    New,
    Not(Box<SearchParam>),
    Old,
    On(DateSpec),
    Or(Box<SearchParam>, Box<SearchParam>),
    Recent,
    Seen,
    SentBefore(DateSpec),
    SentOn(DateSpec),
    SentSince(DateSpec),
    Since(DateSpec),
    Smaller(usize),
    Subject(String),
    Text(String),
    To(String),
    Uid(SeqSet),
    Unanswered,
    Undeleted,
    Undraft,
    Unflagged,
    Unkeyword(String),
    Unseen,
    SavedBefore(DateSpec),
    SavedOn(DateSpec),
    SavedSince(DateSpec),
}

impl SearchParam {
    fn to_dc_name(&self) -> String {
        self.to_string().to_uppercase()
    }

    pub fn to_params(&self) -> Vec<String> {
        match self {
            SearchParam::SequenceSet(set) => vec![set.to_param()],
            SearchParam::All => vec![self.to_dc_name()],
            SearchParam::Answered => vec![self.to_dc_name()],
            SearchParam::Bcc(comp) => vec![self.to_dc_name(), comp.to_owned()],
            SearchParam::Body(comp) => vec![self.to_dc_name(), comp.to_owned()],
            SearchParam::Before(date) => vec![self.to_dc_name(), date.to_param()],
            SearchParam::CC(comp) => vec![self.to_dc_name(), comp.to_owned()],
            SearchParam::Deleted => vec![self.to_dc_name()],
            SearchParam::Draft => vec![self.to_dc_name()],
            SearchParam::Flagged => vec![self.to_dc_name()],
            SearchParam::From(comp) => vec![self.to_dc_name(), comp.to_owned()],
            SearchParam::Header(hdr, comp) => {
                if let Some(comp) = comp {
                    vec![self.to_dc_name(), hdr.to_string(), comp.to_string()]
                } else {
                    vec![self.to_dc_name(), hdr.to_string()]
                }
            }
            SearchParam::Keyword(comp) => vec![self.to_dc_name(), comp.to_owned()],
            SearchParam::Larger(size) => vec![self.to_dc_name(), size.to_string()],
            SearchParam::Mailbox(comp) => vec![self.to_dc_name(), comp.to_owned()],
            SearchParam::MailboxGuid(comp) => vec![self.to_dc_name(), comp.to_owned()],
            SearchParam::New => vec![self.to_dc_name()],
            SearchParam::Not(params) => {
                let mut res = vec![self.to_dc_name()];
                res.append(&mut (*params).to_params());
                res
            }
            SearchParam::Old => vec![self.to_dc_name()],
            SearchParam::On(date) => vec![self.to_dc_name(), date.to_param()],
            SearchParam::Or(param1, param2) => {
                let mut res = param1.to_params();
                res.push(self.to_dc_name());
                res.append(&mut (*param2).to_params());
                res
            }
            SearchParam::Recent => vec![self.to_dc_name()],
            SearchParam::Seen => vec![self.to_dc_name()],
            SearchParam::SentBefore(date) => vec![self.to_dc_name(), date.to_param()],
            SearchParam::SentOn(date) => vec![self.to_dc_name(), date.to_param()],
            SearchParam::SentSince(date) => vec![self.to_dc_name(), date.to_param()],
            SearchParam::Since(date) => vec![self.to_dc_name(), date.to_param()],
            SearchParam::Smaller(size) => vec![self.to_dc_name(), size.to_string()],
            SearchParam::Subject(comp) => vec![self.to_dc_name(), comp.to_owned()],
            SearchParam::Text(comp) => vec![self.to_dc_name(), comp.to_owned()],
            SearchParam::To(comp) => vec![self.to_dc_name(), comp.to_owned()],
            SearchParam::Uid(set) => vec![self.to_dc_name(), set.to_param()],
            SearchParam::Unanswered => vec![self.to_dc_name()],
            SearchParam::Undeleted => vec![self.to_dc_name()],
            SearchParam::Undraft => vec![self.to_dc_name()],
            SearchParam::Unflagged => vec![self.to_dc_name()],
            SearchParam::Unkeyword(comp) => vec![self.to_dc_name(), comp.to_owned()],
            SearchParam::Unseen => vec![self.to_dc_name()],
            SearchParam::SavedBefore(date) => vec![self.to_dc_name(), date.to_param()],
            SearchParam::SavedOn(date) => vec![self.to_dc_name(), date.to_param()],
            SearchParam::SavedSince(date) => vec![self.to_dc_name(), date.to_param()],
        }
    }
}

#[derive(Debug)]
pub struct DateSpec(NaiveDate);

impl DateSpec {
    pub fn from_ymd(year: i32, month: u32, day: u32) -> DateSpec {
        DateSpec(NaiveDate::from_ymd(year, month, day))
    }
    pub fn today() -> DateSpec {
        DateSpec(chrono::Local::now().date_naive())
    }
}

impl ToParam for DateSpec {
    fn to_param(&self) -> String {
        self.0.format("%Y-%m-%d").to_string()
    }
}

/*
  sequence-set
   Matches messages with the given sequence numbers. The sequence-set may be a single UID.  Can be a sequence range, written as from:to, e.g. 100:125.  As comma separated list of sequences, e.g.
   11,50,4.  It's also possible to combine multiple sequences, e.g.  1,3,5,7,10:20.  Using * selects the last mail in the mailbox.
   For example 1:100 matches the first 100 mails and 101:200 the next second hundred mails. 1,5,* matches the first, the fifth and the last email.
*/

#[derive(Debug)]
pub struct SeqSet(Vec<SeqElement>);

impl SeqSet {
    pub fn new(el: SeqElement) -> Self {
        Self(vec![el])
    }
    pub fn add(&mut self, el: SeqElement) {
        self.0.push(el)
    }
}

impl ToParam for SeqSet {
    fn to_param(&self) -> String {
        if let Some(first) = self.0.get(0) {
            let mut res = first.to_param();
            for el in self.0.iter().skip(1) {
                res.push(',');
                res.push_str(el.to_param().as_str());
            }
            res
        } else {
            "".to_owned()
        }
    }
}

#[derive(Debug)]
pub enum SeqElement {
    Uid(usize),
    Range(usize, usize),
    Last,
}

impl ToParam for SeqElement {
    fn to_param(&self) -> String {
        match self {
            SeqElement::Range(start, end) => format!("{}:{}", start, end),
            SeqElement::Last => "*".to_string(),
            SeqElement::Uid(id) => id.to_string(),
        }
    }
}
