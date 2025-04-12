use anyhow::anyhow;
use log::debug;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use tempfile::NamedTempFile;

use crate::config::{InputMode, PresetConfig};

pub fn run_command(cfg: &PresetConfig, input: &str) -> anyhow::Result<(Command, String, String)> {
    let (cmd, stdout, stderr, success) = match cfg.input_mode {
        InputMode::Stdin => run_command_with_stdin(&cfg.command, input, &cfg.language)?,
        InputMode::File => {
            let tmp = NamedTempFile::new()?;
            fs::write(tmp.path(), input)?;
            let args = expand_command_vec(&cfg.command, Some(tmp.path()), &cfg.language);
            run_command_with_file(args)?
        }
    };

    if !success {
        return Err(anyhow!(
            "Error running command {:?}, {} {}",
            cmd,
            stdout,
            stderr
        ));
    }

    Ok((cmd, stdout, stderr))
}

fn run_command_with_stdin(
    command_template: &[String],
    input: &str,
    lang: &str,
) -> anyhow::Result<(Command, String, String, bool)> {
    let args = expand_command_vec(command_template, None, lang);
    let mut cmd = Command::new(&args[0]);
    cmd.args(&args[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn()?;
    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(input.as_bytes())?;
    }
    let output = child.wait_with_output()?;

    Ok((
        cmd,
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.success(),
    ))
}

fn run_command_with_file(args: Vec<String>) -> anyhow::Result<(Command, String, String, bool)> {
    let mut cmd = Command::new(&args[0]);
    cmd.args(&args[1..])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    debug!("Executing command {:?}", args);
    let output = cmd.output()?;

    Ok((
        cmd,
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.success(),
    ))
}

fn expand_command_vec(template: &[String], file: Option<&Path>, lang: &str) -> Vec<String> {
    template
        .iter()
        .map(|arg| {
            let replaced = arg.replace("{lang}", lang);
            if let Some(file) = file {
                replaced
                    .replace("{file}", file.to_str().unwrap_or("{file}"))
                    .replace(
                        "{basename}",
                        file.file_name().and_then(|s| s.to_str()).unwrap_or(""),
                    )
                    .replace(
                        "{dirname}",
                        file.parent().and_then(|s| s.to_str()).unwrap_or(""),
                    )
                    .replace(
                        "{suffix}",
                        file.extension().and_then(|s| s.to_str()).unwrap_or(""),
                    )
                    .replace("{tmpdir}", std::env::temp_dir().to_str().unwrap_or(""))
            } else {
                replaced
            }
        })
        .collect()
}
