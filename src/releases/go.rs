use crate::step::StepError;
use semver::Version;

pub(crate) fn set_version(go_mod: String, new_version: &Version) -> Result<String, StepError> {
    if new_version.major == 0 || new_version.major == 1 {
        // These major versions aren't recorded in go.mod
        return Ok(go_mod);
    }

    let module_line = go_mod
        .lines()
        .find(|line| line.starts_with("module "))
        .ok_or(StepError::MissingModuleLine)?;
    let module = module_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| StepError::MalformedModuleLine(String::from(module_line)))?;
    let mut parts: Vec<&str> = module.split('/').collect();
    let last_part = parts
        .last()
        .ok_or_else(|| StepError::MalformedModuleLine(String::from(module_line)))?;
    let existing_version = if let Some(major_string) = last_part.strip_prefix('v') {
        if let Ok(major) = major_string.parse::<u64>() {
            Some(major)
        } else {
            None
        }
    } else {
        None
    };
    if let Some(existing_version) = existing_version {
        if existing_version == new_version.major {
            // Major version has not changed—keep existing content
            return Ok(go_mod);
        }
        let index = parts.len() - 1;
        let new_version_string = format!("v{}", new_version.major);
        parts[index] = new_version_string.as_str();
        let new_module_line = format!("module {}", parts.join("/"));
        Ok(go_mod.replace(module_line, &new_module_line))
    } else {
        // No existing version found—add new line
        let new_module_line = format!("module {}/v{}", module, new_version.major);
        Ok(go_mod.replace(module_line, &new_module_line))
    }
}
