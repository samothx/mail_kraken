use crate::doveadm::parser::{FetchRecord, FlagsParser, GenericParser, HdrParser, Parser};
use anyhow::{anyhow, Context, Result};
use log::debug;
use std::io::{BufRead, BufReader, Read, Stdin};
use std::process::{Child, ChildStdout, Command, ExitStatus, Stdio};
use std::string::ToString;
use strum_macros;

const MB_SIZE: usize = 1024 * 1024;
const DOVEADM_CMD: &str = "doveadm";

const LINE_FEED: char = 0xAu8 as char;
const FORM_FEED: char = 0xCu8 as char;

mod cmd_args;
pub use cmd_args::CmdArgs;

mod params;
pub use params::{FetchParams, ImapField, SearchParam};

mod parser;

pub struct DoveadmFetch {
    params: FetchParams,
    child: Child,
    stdout: BufReader<Box<dyn Read>>,
    line_count: usize,
    buffer: String,
    parsers: Vec<Box<dyn Parser>>,
}

impl DoveadmFetch {
    pub fn new(params: FetchParams) -> Result<DoveadmFetch> {
        debug!(
            "DoveadmFetch::new: spawning command: {} params: {:?}",
            DOVEADM_CMD,
            params.to_args()
        );
        let mut child = Command::new(DOVEADM_CMD)
            .args(params.to_args()?)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit()) // TODO: do something with this ?
            .spawn()
            .with_context(|| "failed to spawn doveadm fetch command".to_owned())?;

        let mut stdout = match child.stdout.take() {
            Some(stdout) => BufReader::new(Box::new(stdout) as Box<dyn Read>),
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
                ImapField::Hdr => Box::new(HdrParser::new()?) as Box<dyn Parser>,
                _ => Box::new(GenericParser::new(field)?) as Box<dyn Parser>,
            });
        }

        Ok(DoveadmFetch {
            params,
            child,
            stdout,
            line_count: 0,
            buffer: String::new(),
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
        FetchRecord::parse(
            &self.parsers,
            &mut Reader::new(&mut self.stdout, &mut self.buffer, &mut self.line_count),
        )
    }

    fn flush_stdout(&mut self) -> Result<()> {
        let mut buf = vec![0u8; MB_SIZE];
        while self
            .stdout
            .read(&mut buf[..])
            .with_context(|| "failed to read from doveadm fetch stdout")?
            > 0
        {}
        Ok(())
    }
}

impl Drop for DoveadmFetch {
    fn drop(&mut self) {
        // make sure stdout is flushed so process can terminate
        let _ = self.get_exit_status();
    }
}

pub struct Reader<'a> {
    stream: &'a mut BufReader<Box<dyn Read>>,
    buffer: &'a mut String,
    line_count: &'a mut usize,
    consumed: bool,
}

impl<'a> Reader<'a> {
    pub fn new(
        stream: &'a mut BufReader<Box<dyn Read>>,
        buffer: &'a mut String,
        line_count: &'a mut usize,
    ) -> Reader<'a> {
        Reader {
            stream,
            buffer,
            line_count,
            consumed: true,
        }
    }

    fn unconsume(&mut self) {
        self.consumed = false;
    }

    fn next_line(&mut self) -> Result<Option<&str>> {
        if !self.consumed {
            Ok(Some(&self.buffer))
        } else {
            self.buffer.clear();
            if self
                .stream
                .read_line(&mut self.buffer)
                .with_context(|| "failed to read line from doveadm fetch stdout".to_owned())?
                == 0
            {
                Ok(None)
            } else {
                *self.line_count += 1;
                Ok(Some(self.buffer))
            }
        }
    }

    #[allow(dead_code)]
    fn expect_get_line(&mut self) -> Result<&str> {
        if let Some(res) = self.next_line()? {
            Ok(res)
        } else {
            Err(anyhow!("encountered unexpected EOI"))
        }
    }

    #[allow(dead_code)]
    fn line_count(&self) -> usize {
        *self.line_count
    }
}
