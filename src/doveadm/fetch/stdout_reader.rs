use anyhow::{anyhow, Result};
use log::trace;
use regex::bytes::Regex;
use tokio::io::AsyncReadExt;
use tokio::process::ChildStdout;

const BUFF_SIZE: usize = 1024 * 1024;
const STR_BUFF_SIZE: usize = 1024 * 64; // 64K

pub struct StdoutLineReader {
    buffer: Box<[u8; BUFF_SIZE]>,
    line_buf: Vec<u8>,
    str_buf: String,
    consumed: bool,
    finished: bool,
    stream: ChildStdout,
    line_count: usize,
    read_pos: usize,
    end_pos: usize,
}
impl StdoutLineReader {
    pub fn new(stream: ChildStdout) -> StdoutLineReader {
        StdoutLineReader {
            stream,
            read_pos: BUFF_SIZE,
            end_pos: BUFF_SIZE,
            buffer: Box::new([0; BUFF_SIZE]),
            line_count: 0,
            consumed: true,
            finished: false,
            line_buf: Vec::with_capacity(STR_BUFF_SIZE),
            str_buf: String::new(),
        }
    }

    pub(crate) fn unconsume(&mut self) {
        self.consumed = false;
    }

    pub(crate) async fn next_line_raw(&mut self) -> Result<Option<&[u8]>> {
        trace!("next_line_raw: called");
        if !self.consumed {
            trace!("next_line_raw: returning unconsumed buffer");
            self.consumed = true;
            Ok(Some(&self.line_buf[..]))
        } else if self.finished {
            trace!("next_line_raw: stream is finished");
            Ok(None)
        } else {
            self.line_buf.clear();
            loop {
                if self.read_pos < self.end_pos {
                    trace!("next_line_raw: parsing buffer");
                    let mut count = 0;
                    let found = self.buffer[self.read_pos..self.end_pos]
                        .iter()
                        .enumerate()
                        .any(|(idx, ch)| {
                            if *ch == 0xA {
                                count = idx;
                                true
                            } else {
                                false
                            }
                        });

                    if found {
                        // found before the end of the buffer
                        trace!(
                            "next_line_raw: found @offset {}, {} bytes ",
                            self.read_pos,
                            count
                        );
                        if count > 0 {
                            self.line_buf.extend_from_slice(
                                &self.buffer[self.read_pos..self.read_pos + count],
                            );
                        }
                        self.read_pos += count + 1;
                        self.line_count += 1;
                        // TODO: decode RFC2047
                        return Ok(Some(&self.line_buf[..]));
                    } else {
                        // not found before the end of the buffer
                        trace!("next_line_raw: not found, flushing buffer to line buffer");
                        self.line_buf
                            .extend_from_slice(&self.buffer[self.read_pos..self.end_pos]);
                        self.read_pos = self.end_pos;
                    }
                } else {
                    trace!("next_line_raw: filling buffer");
                    self.end_pos = self.stream.read(&mut self.buffer[0..BUFF_SIZE]).await?;
                    if self.end_pos == 0 {
                        self.finished = true;
                        trace!("next_line_raw: stream is finished");
                        if self.line_buf.is_empty() {
                            trace!("next_line_raw: return None");
                            return Ok(None);
                        } else {
                            self.line_count += 1;
                            trace!("next_line_raw: return previously parsed bytes");
                            return Ok(Some(&self.line_buf[..]));
                        }
                    } else {
                        trace!("next_line_raw: buffer refilled to {}", self.end_pos);
                        self.read_pos = 0;
                    }
                }
            }
        }
    }

    pub(crate) async fn next_line(&mut self) -> Result<Option<&str>> {
        trace!("next_line: called");
        if !self.consumed {
            self.consumed = true;
            Ok(Some(self.str_buf.as_str()))
        } else if self.next_line_raw().await?.is_some() {
            self.str_buf = (&*String::from_utf8_lossy(&self.line_buf[..])).to_owned();
            Ok(Some(self.str_buf.as_str()))
        } else {
            Ok(None)
        }
    }

    pub(crate) async fn flush(&mut self) -> Result<()> {
        self.finished = true;
        while self.stream.read(&mut self.buffer[0..]).await? > 0 {}
        Ok(())
    }

    #[allow(dead_code)]
    async fn expect_get_line(&mut self) -> Result<&str> {
        if let Some(res) = self.next_line().await? {
            Ok(res)
        } else {
            Err(anyhow!("encountered unexpected EOI"))
        }
    }

    #[allow(dead_code)]
    fn line_count(&self) -> usize {
        self.line_count
    }
}
