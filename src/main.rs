use clap::{App, Arg, SubCommand};
use colored::*;
use std::ffi::OsStr;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::exit;
use std::process::Command;
use tempfile::tempdir;
use which::which;

fn generate_manifest() -> String {
    return String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly manifestVersion="1.0" xmlns="urn:schemas-microsoft-com:asm.v1">
  <application>
    <windowsSettings>
      <activeCodePage xmlns="http://schemas.microsoft.com/SMI/2019/WindowsSettings">UTF-8</activeCodePage>
    </windowsSettings>
  </application>
</assembly>
"#,
    );
}

fn create_manifest_file(outpath: &Path) -> Result<(), std::io::Error> {
    let manifest = File::create(outpath);
    return manifest.and_then(|mut m| m.write_all(generate_manifest().as_bytes()));
}

macro_rules! requires_mt {
    () => {
        if which("mt").is_err() {
            eprintln!(
                "{}: {} is not in PATH.  Run this tool from e.g. Native Tools Command Prompt.",
                "error".red(),
                "mt".green()
            );
            exit(1);
        }
    };
}

macro_rules! error_and_exit {
    ($err:expr) => {
        eprintln!("{}: {}", "error".red(), $err);
        std::process::exit(1)
    };
}

macro_rules! print_note {
    ($note:expr) => {
        eprintln!("{}: {}", "note".green(), $note)
    };
}

fn main() {
    if !cfg!(windows) {
        eprintln!(
            "{}: You do not have to run this tool outside of Windows.  existing.",
            "error".red()
        );
        exit(1);
    }

    let matches = App::new("forceu8exe")
        .version(&clap::crate_version!()[..])
        .subcommand(
            SubCommand::with_name("apply").arg(Arg::with_name("exepath").required(true).index(1)),
        )
        .subcommand(
            SubCommand::with_name("manifest")
                .arg(Arg::with_name("output").required(true).index(1))
                .arg(Arg::with_name("force").long("force").short("f")),
        )
        .subcommand(
            SubCommand::with_name("apply-manifest")
                .arg(Arg::with_name("in").required(true).index(1))
                .arg(Arg::with_name("out").index(2))
                .arg(Arg::with_name("force").long("force").short("-f")),
        )
        .get_matches();
    if matches.subcommand_name().is_none() {
        error_and_exit!(format!(
            "This tool requires a subcommand.  Try {} to get the help.",
            "-h".green()
        ));
    }

    if let Some(ref matches) = matches.subcommand_matches("apply") {
        requires_mt!();
        let exepath = Path::new(matches.value_of_os("exepath").unwrap());
        if !exepath.exists() {
            error_and_exit!(format!(
                "{} doesn't exist.",
                exepath.to_string_lossy().green()
            ));
        }
        if exepath.extension().unwrap_or_default() != OsStr::new("exe") {
            error_and_exit!(format!(
                "{} doen't end with {}.",
                exepath.to_string_lossy().green(),
                ".exe".green()
            ));
        }
        let working_dir = tempdir().unwrap();
        let manifest_filepath = working_dir.path().join(format!(
            "{}{}",
            &exepath.file_name().unwrap().to_string_lossy(),
            ".manifest"
        ));
        create_manifest_file(&manifest_filepath).unwrap();
        // No manifest -> returns 31 / valid manifest exists -> returns 0
        let validate_manifest_status = Command::new("mt")
            .args(&[
                "-nologo",
                &format!("-inputresource:{}", &exepath.to_string_lossy()),
                "-validate_manifest",
            ])
            .status()
            .unwrap();
        let action = if validate_manifest_status.success() {
            "update"
        } else {
            print_note!("no valid manifest is found in this executable.  Embedding a manifest as the first....");
            "output"
        };
        let mut embed_manifest_result = Command::new("mt")
            .args(&[
                "-nologo",
                "-manifest",
                &manifest_filepath.to_string_lossy(),
                &format!("-{}resource:{}", &action, &exepath.to_string_lossy()),
            ])
            .spawn();
        match embed_manifest_result {
            Ok(ref mut psinfo) => {
                psinfo.wait().unwrap();
                println!(
                    "{}{}",
                    "Succeeded to embed in: ".green(),
                    exepath.to_string_lossy()
                )
            }
            Err(ref err) => {
                error_and_exit!(err);
            }
        }
    } else if let Some(ref matches) = matches.subcommand_matches("manifest") {
        let outputpath = Path::new(matches.value_of_os("output").unwrap());
        let overwriting_allowed = matches.is_present("force");
        if outputpath.is_dir() {
            error_and_exit!(format!(
                "{} is a directory.  Pass a different path",
                outputpath.to_string_lossy().green())
            );
        }
        if outputpath.is_file() && !overwriting_allowed {
            error_and_exit!(format!(
                "{} exists.  Add {} option if you'd like to override it.",
                outputpath.to_string_lossy().green(),
                "-f".green(),
            ));
        }
        match create_manifest_file(&outputpath) {
            Err(err) => {
                error_and_exit!(err);
            }
            _ => {}
        }
        print_note!(format!(
            "succeeded to write the manifest to {}.",
            outputpath.to_string_lossy().green()
        ));
    }
}
