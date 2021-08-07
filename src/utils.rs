use colored::*;
use std::process;
use std::{error::Error, fmt};

pub type DynError = Box<dyn std::error::Error>;

// Logging
fn log(kind: ColoredString, message: &str) {
	println!("{}: {}", kind, message);
}

pub fn log_err(message: &str) {
	log("ERR".red(), message);
}

pub fn log_warn(message: &str) {
	log("WARN".yellow(), message);
}

// Error Handling
#[derive(Debug)]
pub struct CmError {
	pub severity: Severity,
	pub message: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Severity {
	Insignificant,
	Warning,
	Fatal,
}

impl Error for CmError {}

impl fmt::Display for CmError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", self.message)
	}
}

pub fn handle_err(err: &DynError) {
	match err.downcast_ref::<CmError>() {
		Some(cm_error) => match cm_error.severity {
			Severity::Insignificant => {}

			Severity::Warning => {
				log_warn(cm_error.message.as_str());
			}

			Severity::Fatal => {
				log_err(cm_error.message.as_str());
				process::exit(1);
			}
		},

		None => log_err(format!("{}", err).as_str()),
	}
}

// Miscellaneous
pub fn handle_attr(
	attrs: quick_xml::events::attributes::Attributes,
) -> Result<(String, String), DynError> {
	let mut caf_path: Option<String> = None;
	let mut fs_path: Option<String> = None;

	for attr_r in attrs {
		let attr = attr_r?;

		match attr.key {
			b"path" => {
				fs_path = Some(String::from_utf8(attr.value.to_vec())?);
			}

			b"caf" => {
				caf_path = Some(String::from_utf8(attr.value.to_vec())?);
			}

			_ => {}
		}
	}

	Ok((
		caf_path.ok_or(Box::new(CmError {
			severity: Severity::Insignificant,
			message: String::from("tag does not contain the `caf` attribute"),
		}))?,
		fs_path.ok_or(Box::new(CmError {
			severity: Severity::Insignificant,
			message: String::from("tag does not contain the `path` attribute"),
		}))?,
	))
}
