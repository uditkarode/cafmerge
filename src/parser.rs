use crate::utils::{self, DynError};

use colored::Colorize;
use quick_xml::{events::Event, Reader};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

pub fn parse_xml(xmlp: &str) -> Result<(Vec<String>, Vec<String>), DynError> {
    let xml_path = Path::new(xmlp);

    if !xml_path.exists() {
        return Err(Box::new(utils::CmError {
            severity: utils::Severity::Fatal,
            message: format!("cannot access '{}': No such file", xmlp),
        }));
    }

    if xml_path.is_dir() {
        return Err(Box::new(utils::CmError {
            severity: utils::Severity::Fatal,
            message: format!("cafmerge: '{}': Is a directory", xmlp),
        }));
    }

    let mut reader: Reader<BufReader<File>> = match Reader::from_file(xml_path) {
        Ok(r) => r,
        Err(e) => {
            return Err(Box::new(utils::CmError {
                severity: utils::Severity::Fatal,
                message: format!("{}", e),
            }));
        }
    };

    reader.trim_text(true);
    reader.check_comments(false);

    let mut buf: Vec<u8> = Vec::new();

    let mut paths: Vec<String> = Vec::new();
    let mut fs_paths: Vec<String> = Vec::new();

    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e)) => {
                let tag = e.name();

                let (kind, to_push) = if tag == b"remove-project" {
                    (utils::TagKind::RemoveProject, &mut paths)
                } else if tag == b"project" {
                    (utils::TagKind::Project, &mut fs_paths)
                } else {
                    continue;
                };

                for attr in e.attributes() {
                    if let Ok(path_str) = utils::handle_attr(attr, &kind) {
                        to_push.push(path_str);
                    }
                }
            }

            Ok(Event::Eof) => break,

            Err(e) => {
                return Err(Box::new(utils::CmError {
                    severity: utils::Severity::Fatal,
                    message: format!("couldn't parse xml: {}", e),
                }));
            }

            _ => (),
        }

        buf.clear();
    }

    if paths.len() != fs_paths.len() {
        return Err(Box::new(utils::CmError {
            severity: utils::Severity::Fatal,
            message: format!(
                "{} and {} entries are not in sync",
                "remove-project".bold(),
                "project".bold()
            ),
        }));
    }

    Ok((paths, fs_paths))
}
