use crate::utils::{self, DynError};
use quick_xml::{events::Event, Reader};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

pub struct MergeItem {
    pub caf_path: String,
    pub fs_path: String,
}

pub fn parse_xml(xmlp: &str) -> Result<Vec<MergeItem>, DynError> {
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
    let mut paths: Vec<MergeItem> = Vec::new();

    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Empty(ref e)) => {
                if e.name() == b"project" {
                    if let Ok((caf_path_str, fs_path_str)) = utils::handle_attr(e.attributes()) {
                        paths.push(MergeItem {
                            caf_path: caf_path_str,
                            fs_path: fs_path_str,
                        });
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

    Ok(paths)
}
