use colored::*;
use quick_xml::events::attributes::Attribute;
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

pub enum TagKind {
	RemoveProject,
	Project,
}

// Miscellaneous
pub fn handle_attr(
	attr_r: Result<Attribute<'_>, quick_xml::Error>,
	kind: &TagKind,
) -> Result<String, DynError> {
	let attr = attr_r?;

	let attr_name = match kind {
		TagKind::RemoveProject => b"name",
		TagKind::Project => b"path",
	};

	return if attr.key == attr_name {
		let s = String::from_utf8(attr.value.to_vec())?;
		Ok(s)
	} else {
		Err(Box::new(CmError {
			severity: Severity::Insignificant,
			message: format!(
				"tag does not contain the field '{}'",
				String::from_utf8(attr_name.to_vec())?
			),
		}))
	};
}
