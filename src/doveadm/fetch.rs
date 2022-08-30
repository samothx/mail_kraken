use anyhow::{anyhow, Context, Result};
use log::{debug, error, trace};
use std::process::{ExitStatus, Stdio};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::{Child, ChildStdout, Command};
use tokio::task::JoinHandle;

use super::{DOVEADM_CMD, MB_SIZE};
use crate::switch_to_user;
use params::{FetchParams, ImapField};
use parser::{GenericParser, HdrParser, Parser};

mod stdout_reader;

pub mod params;
mod parser;
use crate::doveadm::fetch::parser::{
    DateReceivedParser, DateSavedParser, DateSentParser, FlagsParser, GuidParser, MailboxParser,
    SizePhysicalParser, UidParser,
};

const STDOUT_BUF_SIZE: usize = 1024 * 1024 * 100; // 10MB

use crate::doveadm::fetch::stdout_reader::StdoutLineReader;
pub use parser::{FetchFieldRes, FetchRecord};

pub struct Fetch {
    params: FetchParams,
    child: Child,
    stdout: StdoutLineReader,
    // stderr_task: JoinHandle<()>,
    line_count: usize,
    buffer: String,
    parsers: Vec<Box<dyn Parser + Sync + Send>>,
}

impl Fetch {
    pub fn new(params: FetchParams) -> Result<Fetch> {
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

        /*        let stderr_task = match child.stderr.take() {
                    Some(stdout) => {
                        let mut reader = BufReader::with_capacity(STDOUT_BUF_SIZE, stdout);
                        tokio::spawn(async move {
                            let mut line = String::new();
                            while let Ok(bytes) = reader.read_line(&mut line).await {
                                if bytes > 0 {
                                    error!("fetch stderr: {}", line);
                                    line.clear();
                                } else {
                                    break;
                                }
                            }
                        })
                    }
                    None => {
                        return Err(anyhow!(
                            "unable to retrieve stdout handle for fetch command"
                        ))
                    }
                };
        */
        let mut parsers: Vec<Box<dyn Parser + Sync + Send>> = Vec::new();
        for field in params.fields() {
            parsers.push(match field {
                ImapField::Flags => Box::new(FlagsParser::new()?) as Box<dyn Parser + Sync + Send>,
                ImapField::Uid => Box::new(UidParser::new()?) as Box<dyn Parser + Sync + Send>,
                ImapField::Guid => Box::new(GuidParser::new()?) as Box<dyn Parser + Sync + Send>,
                ImapField::Mailbox => {
                    Box::new(MailboxParser::new()?) as Box<dyn Parser + Sync + Send>
                }
                ImapField::DateSent => {
                    Box::new(DateSentParser::new()?) as Box<dyn Parser + Sync + Send>
                }
                ImapField::DateReceived => {
                    Box::new(DateReceivedParser::new()?) as Box<dyn Parser + Sync + Send>
                }
                ImapField::DateSaved => {
                    Box::new(DateSavedParser::new()?) as Box<dyn Parser + Sync + Send>
                }
                ImapField::SizePhysical => {
                    Box::new(SizePhysicalParser::new()?) as Box<dyn Parser + Sync + Send>
                }

                /*ImapField::DateReceived | ImapField::DateSaved | ImapField::DateSent => {
                    Box::new(SingleLineParser::new(field, true)?) as Box<dyn Parser + Sync>
                }*/
                ImapField::Hdr => Box::new(HdrParser::new()?) as Box<dyn Parser + Sync + Send>,
                _ => Box::new(GenericParser::new(field)?) as Box<dyn Parser + Sync + Send>,
            });
        }

        Ok(Fetch {
            params,
            child,
            stdout: StdoutLineReader::new(stdout),
            line_count: 0,
            buffer: String::new(),
            parsers,
        })
    }

    pub async fn get_exit_status(&mut self) -> Result<ExitStatus> {
        self.flush_stdout().await?;
        self.child
            .wait()
            .await
            .with_context(|| "failed to wait for dveadm fetch to terminate".to_owned())
    }

    pub async fn parse_record(&mut self) -> Result<Option<FetchRecord>> {
        debug!("parse_record: called");
        FetchRecord::parse(&self.parsers, &mut self.stdout).await
    }

    async fn flush_stdout(&mut self) -> Result<()> {
        let mut buf = vec![0u8; MB_SIZE];
        self.stdout.flush().await?;
        Ok(())
    }
}

impl Drop for Fetch {
    fn drop(&mut self) {
        // make sure stdout is flushed so process can terminate
        let _ = self.get_exit_status();
    }
}
