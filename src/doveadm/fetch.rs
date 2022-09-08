use anyhow::{anyhow, Context, Result};
use log::debug;
use std::fs::File;
use std::process::{Child, Command, ExitStatus, Stdio};

use super::DOVEADM_CMD;
use crate::switch_to_user;
use params::{FetchParams, ImapField};
use parser::{HdrParser, Parser};

mod stdout_reader;

pub mod params;
mod parser;
use crate::doveadm::fetch::parser::{
    DateReceivedParser, DateSavedParser, DateSentParser, FlagsParser, GuidParser, MailboxParser,
    SizePhysicalParser, UidParser,
};

use crate::doveadm::fetch::stdout_reader::StdoutLineReader;
pub use parser::{FetchFieldRes, FetchRecord};

pub struct Fetch {
    // params: FetchParams,
    child: Child,
    stdout: StdoutLineReader,
    parsers: Vec<Box<dyn Parser>>,
}

impl Fetch {
    pub fn new(params: FetchParams, copy_to: Option<File>) -> Result<Fetch> {
        debug!(
            "DoveadmFetch::new: spawning command: {} params: {:?}",
            DOVEADM_CMD,
            params.to_args()
        );
        // TODO: set userid to root for this
        switch_to_user(true)?;
        let mut child = Command::new(DOVEADM_CMD)
            .args(params.to_args()?)
            // .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit()) // TODO: do something with this ?
            .spawn()
            .with_context(|| "failed to spawn doveadm fetch command".to_owned())?;
        // TODO: set userid back to nobody
        switch_to_user(false)?;
        let stdout = match child.stdout.take() {
            Some(stdout) => stdout,
            None => {
                return Err(anyhow!(
                    "unable to retrieve stdout handle for fetch command"
                ))
            }
        };

        let mut parsers: Vec<Box<dyn Parser>> = Vec::new();
        for field in params.fields() {
            parsers.push(match field {
                ImapField::Flags => Box::new(FlagsParser::new()?) as Box<dyn Parser>,
                ImapField::Uid => Box::new(UidParser::new()?) as Box<dyn Parser>,
                ImapField::Guid => Box::new(GuidParser::new()?) as Box<dyn Parser>,
                ImapField::Mailbox => Box::new(MailboxParser::new()?) as Box<dyn Parser>,
                ImapField::DateSent => Box::new(DateSentParser::new()?) as Box<dyn Parser>,
                ImapField::DateReceived => Box::new(DateReceivedParser::new()?) as Box<dyn Parser>,
                ImapField::DateSaved => Box::new(DateSavedParser::new()?) as Box<dyn Parser>,
                ImapField::SizePhysical => Box::new(SizePhysicalParser::new()?) as Box<dyn Parser>,

                /*ImapField::DateReceived | ImapField::DateSaved | ImapField::DateSent => {
                    Box::new(SingleLineParser::new(field, true)?) as Box<dyn Parser + Sync>
                }*/
                ImapField::Hdr => Box::new(HdrParser::new()?) as Box<dyn Parser>,
                _ => {
                    return Err(anyhow!(
                        "no parser found for field: [{}]",
                        field.to_string()
                    ));
                }
            });
        }

        Ok(Fetch {
            // params,
            child,
            stdout: StdoutLineReader::new(stdout, copy_to),
            parsers,
        })
    }

    pub fn get_exit_status(&mut self) -> Result<ExitStatus> {
        self.flush_stdout()?;
        self.child
            .wait()
            .with_context(|| "failed to wait for dveadm fetch to terminate".to_owned())
    }

    pub fn parse_record(&mut self) -> Result<Option<FetchRecord>> {
        debug!("parse_record: called");
        FetchRecord::parse(&self.parsers, &mut self.stdout)
    }

    fn flush_stdout(&mut self) -> Result<()> {
        self.stdout.flush()?;
        Ok(())
    }
}

impl Drop for Fetch {
    fn drop(&mut self) {
        // make sure stdout is flushed so process can terminate
        let _ = self.get_exit_status();
    }
}
