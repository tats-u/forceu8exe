use clap::{App, Arg, SubCommand};
use colored::*;
use std::ffi::OsStr;
use std::fs::File;
use std::path::Path;
use std::process::exit;
use std::process::Command;
use tempfile::tempdir;
use which::which;

use std::io::Write;

fn generate_manifest(exename: &str) -> String {
    return String::from(format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly manifestVersion="1.0" xmlns="urn:schemas-microsoft-com:asm.v1">
  <assemblyIdentity type="win32" name="{}" version="6.0.0.0"/>
  <application>
    <windowsSettings>
      <activeCodePage xmlns="http://schemas.microsoft.com/SMI/2019/WindowsSettings">UTF-8</activeCodePage>
    </windowsSettings>
  </application>
</assembly>
"#,
        exename
    ));
}

fn main() {
    if !cfg!(windows) {
        eprintln!(
            "{}: You do not have to run this tool outside of Windows.  existing.",
            "error".red()
        );
        exit(1);
    }

    if which("mt").is_err() {
        eprintln!(
            "{}: {} is not in PATH.  Run this tool from e.g. Native Tools Command Prompt.",
            "error".red(),
            "mt".green()
        );
        exit(1);
    }
    let matches = App::new("forceu8exe")
        .version(&clap::crate_version!()[..])
        .subcommand(
            SubCommand::with_name("apply").arg(Arg::with_name("exepath").required(true).index(1)),
        )
        .get_matches();
    if matches.subcommand_name().is_none() {
        eprintln!(
            "{}: This tool requires a subcommand.  Try {} to get the help.",
            "error".red(),
            "-h".green()
        );
        exit(1);
    }
    if let Some(matches) = matches.subcommand_matches("apply") {
        let exepath = Path::new(matches.value_of("exepath").unwrap());
        if !exepath.exists() {
            eprintln!(
                "{}: {} doesn't exist.",
                "error".red(),
                exepath.to_string_lossy().green()
            );
            exit(1);
        }
        if exepath.extension().unwrap_or_default() != OsStr::new("exe") {
            eprintln!(
                "{}: {} doen't end with {}.",
                "error".red(),
                exepath.to_string_lossy().green(),
                ".exe".green()
            );
            exit(1);
        }
        let exestem = exepath.file_stem().unwrap().to_string_lossy();
        let working_dir = tempdir().unwrap();
        let manifest_filepath = working_dir.path().join(format!(
            "{}{}",
            &exepath.file_name().unwrap().to_string_lossy(),
            ".manifest"
        ));
        {
            let mut manifest = File::create(&manifest_filepath).unwrap();
            manifest
                .write_all(generate_manifest(exestem.as_ref()).as_bytes())
                .unwrap();
        }
        // No manifest -> 31 / valid manifest exists -> 0
        let validate_manifest_status = Command::new("mt")
            .args(&["-nologo", 
                &format!("-inputresource:{}", &exepath.to_string_lossy()),
                "-validate_manifest"
            ])
            .status().unwrap();
        let action = if validate_manifest_status.success() { "update" } else { "output"} ;
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
                eprintln!("{}: {}", "error".red(), err);
                exit(1);
            }
        }
    }
}
