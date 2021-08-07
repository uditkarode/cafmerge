mod git;
mod parser;
mod utils;

use clap::{App, Arg};
use colored::*;
use std::path::Path;
use std::process;

use crate::git::PullResult;
use crate::utils::handle_err;

pub const CAF_BASE_URL: &str = "https://source.codeaurora.org/quic/la/";

fn main() {
    let matches = App::new(format!("{}", "cafmerge".bold().yellow()))
        .version("0.2")
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
                .required(true),
        )
        .get_matches();

    let xmlp = matches.value_of("manifest").unwrap();
    let tag = matches.value_of("tag").unwrap();

    let paths = match parser::parse_xml(xmlp) {
        Ok(paths) => paths,

        Err(e) => {
            handle_err(&e);
            /* If the XML couldn't be parsed, there's
             * nothing for us to do, so exit prematurely */
            process::exit(1);
        }
    };

    for path in paths.iter() {
        print!("merging into {}... ", path);

        let git_path = Path::new(path);

        if !git_path.exists() {
            utils::log_warn(format!("cannot access {}: No such directory", path).as_str());
            continue;
        }

        if !git_path.is_dir() {
            utils::log_warn(format!("cannot use {}: Is not a directory", path).as_str());
            continue;
        }

        let res = git::pull(path, tag.to_string());

        match res {
            Ok(o) => match o {
                PullResult::Clean => println!("{}", "clean".green()),
                PullResult::Conflicted { conflicted_files } => {
                    println!("{}", format!("{} conflicts", conflicted_files).red())
                }
                PullResult::NothingToDo => println!("{}", "nothing to do".yellow()),
            },
            Err(e) => handle_err(&e),
        }
    }
}
