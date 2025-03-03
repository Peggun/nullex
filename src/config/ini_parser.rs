/*

For future .conf files. Had an idea but then realised theres no point right now.

*/

use alloc::vec::Vec;
use core::str;

#[derive(Debug)]
pub struct IniSection<'a> {
    pub name: &'a str,
    pub properties: Vec<(&'a str, &'a str)>,
}

#[derive(Debug)]
pub struct IniFile<'a> {
    pub sections: Vec<IniSection<'a>>,
}

/// Parses an INI formatted string into an IniFile structure.
pub fn parse_ini(input: &str) -> IniFile {
    let mut sections = Vec::new();
    // Optionally, use a default section if key/value pairs occur outside any section.
    let mut current_section: Option<IniSection> = None;

    for line in input.lines() {
        let line = line.trim();
        // Skip empty or comment lines.
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        // Check if the line is a section header.
        if line.starts_with('[') && line.ends_with(']') {
            // If we were parsing a section, push it to our sections vector.
            if let Some(sec) = current_section.take() {
                sections.push(sec);
            }
            // Extract the section name and trim whitespace.
            let section_name = line[1..line.len()-1].trim();
            current_section = Some(IniSection {
                name: section_name,
                properties: Vec::new(),
            });
        } else if let Some(pos) = line.find('=') {
            // Parse key and value by splitting on the '=' character.
            let key = line[..pos].trim();
            let value = line[pos+1..].trim();
            // If we haven't started a section, create a default section.
            if let Some(ref mut sec) = current_section {
                sec.properties.push((key, value));
            } else {
                current_section = Some(IniSection {
                    name: "", // Default section name can be empty or something like "global".
                    properties: vec![(key, value)],
                });
            }
        }
        // Lines that don't match any expected pattern are ignored.
    }
    // Push the last section if it exists.
    if let Some(sec) = current_section {
        sections.push(sec);
    }
    IniFile { sections }
}