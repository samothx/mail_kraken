use anyhow::{anyhow, Result};
use log::{debug, trace};
use tokio::io::AsyncReadExt;
use tokio::process::ChildStdout;

const BUFF_SIZE: usize = 1024 * 1024;

pub struct StdoutReader {
    buffer: Box<[u8; BUFF_SIZE]>,
    line_buf: String,
    consumed: bool,
    finished: bool,
    stream: ChildStdout,
    line_count: usize,
    read_pos: usize,
    end_pos: usize,
}
impl StdoutReader {
    pub fn new(stream: ChildStdout) -> StdoutReader {
        StdoutReader {
            stream,
            read_pos: BUFF_SIZE,
            end_pos: BUFF_SIZE,
            buffer: Box::new([0; BUFF_SIZE]),
            line_count: 0,
            consumed: true,
            finished: false,
            line_buf: String::with_capacity(1024),
        }
    }

    pub(crate) fn unconsume(&mut self) {
        self.consumed = false;
    }

    pub(crate) async fn next_line(&mut self) -> Result<Option<&str>> {
        debug!("next_line: called");
        if !self.consumed {
            debug!("next_line: returning unconsumed buffer");
            self.consumed = true;
            Ok(Some(self.line_buf.as_str()))
        } else {
            if self.finished {
                debug!("next_line: stream is finished");
                Ok(None)
            } else {
                self.line_buf.clear();
                loop {
                    if self.read_pos < self.end_pos {
                        debug!("next_line: parsing buffer");
                        let mut count = 0;
                        let found = self.buffer[self.read_pos..self.end_pos]
                            .iter()
                            .enumerate()
                            .any(|(idx, ch)| {
                                if *ch == 0xAu8 {
                                    count = idx;
                                    true
                                } else {
                                    false
                                }
                            });

                        if found {
                            // found before the end of the buffer
                            debug!("next_line: found @offset {}", count);
                            if count > 0 {
                                self.line_buf.push_str(&*String::from_utf8_lossy(
                                    &self.buffer[self.read_pos..self.read_pos + count],
                                ));
                            }
                            self.read_pos += count + 1;
                            self.line_count += 1;
                            return Ok(Some(self.line_buf.as_str()));
                        } else {
                            // not found before the end of the buffer
                            self.line_buf.push_str(&*String::from_utf8_lossy(
                                &self.buffer[self.read_pos..self.end_pos],
                            ));
                            self.read_pos = self.end_pos;
                        }
                    } else {
                        self.end_pos = self.stream.read(&mut self.buffer[0..BUFF_SIZE]).await?;
                        if self.end_pos == 0 {
                            self.finished = true;
                            if self.line_buf.is_empty() {
                                return Ok(None);
                            } else {
                                self.line_count += 1;
                                return Ok(Some(self.line_buf.as_str()));
                            }
                        } else {
                            self.read_pos = 0;
                        }
                    }
                }
            }
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
