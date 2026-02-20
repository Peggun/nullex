//!
//! ini_parser.rs
//!
//! For future .conf files. Had an idea but then realised theres no point right now.
//!

use alloc::vec::Vec;
use core::str;

#[derive(Debug)]
struct IniSection<'a> {
	name: &'a str,
	properties: Vec<(&'a str, &'a str)>
}

#[derive(Debug)]
struct IniFile<'a> {
	sections: Vec<IniSection<'a>>
}

/// Parses an INI formatted string into an IniFile structure.
pub fn parse_ini(input: &'_ str) -> IniFile<'_> {
	let mut sections = Vec::new();

	let mut current_section: Option<IniSection> = None;

	for line in input.lines() {
		let line = line.trim();
		// skip empty or comment lines.
		if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
			continue;
		}
		// check if the line is a section header.
		if line.starts_with('[') && line.ends_with(']') {
			// if we were parsing a section, push it to our sections vector.
			if let Some(sec) = current_section.take() {
				sections.push(sec);
			}
			// extract the section name and trim whitespace.
			let section_name = line[1..line.len() - 1].trim();
			current_section = Some(IniSection {
				name: section_name,
				properties: Vec::new()
			});
		} else if let Some(pos) = line.find('=') {
			// parse key and value by splitting on the '=' character.
			let key = line[..pos].trim();
			let value = line[pos + 1..].trim();
			// if we haven't started a section, create a default section.
			if let Some(ref mut sec) = current_section {
				sec.properties.push((key, value));
			} else {
				current_section = Some(IniSection {
					name: "", // default section name can be empty or something like "global".
					properties: vec![(key, value)]
				});
			}
		}
		// lines that don't match any expected pattern are ignored.
	}
	// push the last section if it exists.
	if let Some(sec) = current_section {
		sections.push(sec);
	}
	IniFile {
		sections
	}
}
