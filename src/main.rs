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
    } else if let Some(ref matches) = matches.subcommand_matches("apply-manifest") {
        let input_path_str = matches.value_of_os("in").unwrap();
        let input_path = Path::new(&input_path_str);
        let output_path_option = matches.value_of_os("out");
        let overwriting_allowed = matches.is_present("force");
        if !input_path.is_file() {
            error_and_exit!(format!(
                "The input file {} is not a file.",
                input_path_str.to_string_lossy().green()
            ));
        }
        let output_path_str_ref = match output_path_option {
            Some(ref output_path_str) => &output_path_str,
            None => {
                // Overwriting mode (in = out)
                &input_path_str
            }
        };
        let output_path = Path::new(output_path_str_ref);
        if output_path.exists() && !output_path.is_file() {
            error_and_exit!(format!(
                "It is not allowed to pass a path {} for non-file as output.",
                output_path_str_ref.to_string_lossy().green()
            ));
        }
        if !overwriting_allowed && output_path.exists()
        /* && output_path.is_file() */ // already implied
        {
            error_and_exit!(format!(
                "Overwriting is allowed only when {} option is given.",
                "-f".green()
            ));
        }
        let working_dir = tempdir().unwrap();
        let u8manifest_path = working_dir.path().join("utf8.manifest");
        create_manifest_file(&u8manifest_path).unwrap();
        let mut apply_manifest_result = Command::new("mt")
            .args(&[
                "-nologo",
                "-manifest",
                &input_path.to_string_lossy(),
                &u8manifest_path.to_string_lossy(),
                &format!("-out:{}", &output_path.to_string_lossy()),
            ])
            .spawn();
        match apply_manifest_result {
            Ok(ref mut psinfo) => {
                psinfo.wait().unwrap();
                println!(
                    "{}{}",
                    "Succeeded to generate UTF-8 manifest: ",
                    output_path.to_string_lossy().green()
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
                outputpath.to_string_lossy().green()
            ));
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
