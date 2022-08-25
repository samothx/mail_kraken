use crate::doveadm::DOVEADM_CMD;
use crate::switch_to_user;
use anyhow::{anyhow, Context, Result};
use log::error;
use regex::Regex;
use std::io::BufRead;
use tokio::process::Command;
// doveadm auth login <user> <passwd>

pub async fn authenticate(user: &str, passwd: &str) -> Result<bool> {
    switch_to_user(true)?;
    let output = Command::new(DOVEADM_CMD)
        .args(&["auth", "login", user, passwd])
        .output()
        .await;
    switch_to_user(false)?;
    match output {
        Ok(output) => {
            if !output.status.success() {
                error!(
                    "authenticate: doveadm auth login failed: {:?}",
                    output.status.code()
                );
                output.stdout.lines().for_each(|line| {
                    error!(
                        "authenticate: stdout: {}",
                        line.unwrap_or("error reading line".to_owned())
                    );
                });
                output.stderr.lines().for_each(|line| {
                    error!(
                        "authenticate: stderr: {}",
                        line.unwrap_or("error reading line".to_owned())
                    );
                })
            }
            if let Some(line) = output.stdout.lines().next() {
                match line {
                    Ok(line) => {
                        let regex = Regex::new(r"^([^:]+):\s+(\S+)\s+auth\s(succeeded|failed)$")
                            .with_context(|| "failed to compile regex".to_owned())?;
                        if let Some(captures) = regex.captures(line.as_str()) {
                            match captures.get(3).unwrap().as_str() {
                                "suceeded" => Ok(output.status.success()),
                                _ => Ok(false),
                            }
                        } else {
                            Err(anyhow!(
                                "no match on regex for doveadm auth login output: {}",
                                line
                            ))
                        }
                    }
                    Err(e) => {
                        Err(e).with_context(|| "failed to read line from doveadm output".to_owned())
                    }
                }
            } else {
                Err(anyhow!("Empty output from doveadm auth login"))
            }
        }
        Err(e) => Err(e).with_context(|| "failed to start doveadm process".to_owned()),
    }
}
