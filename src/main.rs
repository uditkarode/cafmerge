mod git;
mod parser;
mod utils;

use clap::{App, Arg};
use colored::Colorize;
use std::path::Path;
use std::process;

use crate::git::GitResult;
use crate::utils::handle_err;

pub const CAF_BASE_URL: &str = "https://source.codeaurora.org/quic/la/";

fn main() {
    let matches = App::new(format!("{}", "cafmerge".bold().yellow()))
        .version("1.0")
        .author(
            format!(
                "{} {} {}",
                "by",
                "Udit Karode".bold(),
                "<udit.karode@gmail.com>"
            )
            .as_str(),
        )
        .about("Merge CAF tags into your ROM source")
        .arg(
            Arg::new("manifest")
                .short('m')
                .long("manifest")
                .value_name(format!("{}", "XML_PATH".bold().yellow()).as_str())
                .about("The manifest file to use")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::new("tag")
                .short('t')
                .long("tag")
                .value_name(format!("{}", "CAF_TAG".bold().yellow()).as_str())
                .takes_value(true)
                .required(false),
        )
        .arg(
            Arg::new("conflicts")
                .short('c')
                .long("show-conflicts")
                .takes_value(false)
                .required(false),
        )
        .get_matches();

    let xmlp = matches.value_of("manifest").unwrap();

    let show_conflicts = matches.is_present("conflicts");

    let tag = if !show_conflicts {
        match matches.value_of("tag") {
            Some(tag_val) => Some(tag_val),
            None => {
                utils::log_err("you must specify a tag");
                process::exit(1);
            }
        }
    } else {
        None
    };

    let paths = match parser::parse_xml(xmlp) {
        Ok(paths) => paths,

        Err(e) => {
            handle_err(&e);
            /* If the XML couldn't be parsed, there's
             * nothing for us to do, so exit prematurely */
            process::exit(1);
        }
    };

    let total = paths.iter().count();

    if total == 0 {
        utils::log_err("could not find any merge targets, did you forget to add the caf attribute to your manifest entries?");
        process::exit(1);
    }

    if show_conflicts {
        println!("Checking for conflicts... \n");

        let mut conflicted = 0;
        for path in paths.iter() {
            let git_path = Path::new(&path.fs_path);

            if !git_path.exists() {
                continue;
            }

            if !git_path.is_dir() {
                continue;
            }

            match git::is_conflicted(&path.fs_path) {
                Ok(GitResult::Conflicted { conflicted_files }) => {
                    conflicted += 1;
                    println!(
                        "{} {}: {} conflicts found",
                        format!("[{}]", conflicted).dimmed(),
                        path.fs_path,
                        format!("{}", conflicted_files).red(),
                    );
                }

                Err(e) => {
                    print!("{}: ", path.fs_path);
                    handle_err(&e);
                }

                _ => {}
            }
        }

        println!(
            "\n{} repositories contain unresolved conflicts",
            format!("{}", conflicted).red().bold()
        );
    } else {
        for (ind, path) in paths.iter().enumerate() {
            println!(
                "{} merging into {}... ",
                format!("[{}/{}]", ind + 1, total).bold().dimmed(),
                path.fs_path
            );

            let git_path = Path::new(&path.fs_path);

            if !git_path.exists() {
                utils::log_warn(
                    format!("cannot access {}: No such directory", path.fs_path).as_str(),
                );
                continue;
            }

            if !git_path.is_dir() {
                utils::log_warn(
                    format!("cannot use {}: Is not a directory", path.fs_path).as_str(),
                );
                continue;
            }

            let res = git::pull(&path.fs_path, &path.caf_path, tag.unwrap().to_string());

            match res {
                Ok(o) => match o {
                    GitResult::Clean => println!("{}", "Merged clean\n".green()),
                    GitResult::Conflicted { conflicted_files } => {
                        println!(
                            "{}",
                            format!("{} conflicts found\n", conflicted_files).red()
                        )
                    }
                    GitResult::NothingToDo => println!("{}", "Nothing to do\n".yellow()),
                },
                Err(e) => handle_err(&e),
            }
        }
    }
}
