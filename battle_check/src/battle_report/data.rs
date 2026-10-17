use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

use anyhow::{Context, Result};
use serde::Serialize;

use super::model::Subject;

pub(crate) const ROOT: &str = "battle-support-data";
const SCHEMA_VERSION: u32 = 1;

#[derive(Serialize)]
struct Manifest {
    schema_version: u32,
    reports: BTreeMap<&'static str, Vec<String>>,
}

pub(crate) fn write_subjects(
    output: &Path,
    kind: &'static str,
    subjects: &[Subject],
    generated: &mut Vec<String>,
    reports: &mut BTreeMap<&'static str, Vec<String>>,
) -> Result<()> {
    let directory = output.join(ROOT).join(kind);
    fs::create_dir_all(&directory)
        .with_context(|| format!("create {kind} report data directory"))?;
    let paths = subjects
        .iter()
        .map(|subject| {
            let relative = format!("{kind}/{}.json", subject.id);
            let path = output.join(ROOT).join(&relative);
            let mut json = serde_json::to_string_pretty(subject)?;
            json.push('\n');
            write_new(&path, &json)?;
            generated.push(format!("{ROOT}/{relative}"));
            Ok(relative)
        })
        .collect::<Result<Vec<_>>>()?;
    reports.insert(kind, paths);
    Ok(())
}

pub(crate) fn write_manifest(
    output: &Path,
    reports: BTreeMap<&'static str, Vec<String>>,
    generated: &mut Vec<String>,
) -> Result<()> {
    let path = output.join(ROOT).join("manifest.json");
    let mut json = serde_json::to_string_pretty(&Manifest {
        schema_version: SCHEMA_VERSION,
        reports,
    })?;
    json.push('\n');
    write_new(&path, &json)?;
    generated.push(format!("{ROOT}/manifest.json"));
    Ok(())
}

fn write_new(path: &Path, contents: &str) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("create report data {}", path.display()))?;
    file.write_all(contents.as_bytes())
        .with_context(|| format!("write report data {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_versioned_subject_modules() {
        let root =
            std::env::temp_dir().join(format!("enigma-battle-report-data-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let subjects = [Subject {
            id: 42,
            name: "Example".to_owned(),
            slug: "example".to_owned(),
            variants: Vec::new(),
            chapter_id: None,
        }];
        let mut generated = Vec::new();
        let mut reports = BTreeMap::new();

        write_subjects(&root, "hero", &subjects, &mut generated, &mut reports).unwrap();
        write_manifest(&root, reports, &mut generated).unwrap();

        let manifest: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(root.join(ROOT).join("manifest.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(manifest["schema_version"], 1);
        assert_eq!(manifest["reports"]["hero"][0], "hero/42.json");
        assert!(root.join(ROOT).join("hero").join("42.json").is_file());

        fs::remove_dir_all(root).unwrap();
    }
}
