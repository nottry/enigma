use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct CapabilityKey {
    pub(crate) domain: &'static str,
    pub(crate) opcode: i32,
    pub(crate) type_name: String,
}

impl CapabilityKey {
    pub(crate) fn new(domain: &'static str, opcode: i32, type_name: impl Into<String>) -> Self {
        Self {
            domain,
            opcode,
            type_name: type_name.into(),
        }
    }
}

#[derive(Default)]
pub(crate) struct Report {
    pub(crate) errors: BTreeSet<String>,
    pub(crate) warnings: BTreeSet<String>,
    // Unproven but not actionable, e.g. wire markers inferred without a capture.
    pub(crate) notes: BTreeSet<String>,
    pub(crate) checked_skills: HashSet<i32>,
    pub(crate) checked_buffs: HashSet<i32>,
    pub(crate) capabilities: BTreeSet<CapabilityKey>,
    pub(crate) gaps: BTreeMap<CapabilityKey, BTreeSet<&'static str>>,
    pub(crate) gap_paths: BTreeMap<CapabilityKey, BTreeSet<String>>,
    pub(crate) explanations: Vec<String>,
    pub(crate) quiet: bool,
    pub(crate) explain: bool,
    pub(crate) wire_evidence: crate::wire_evidence::Evidence,
}

impl Report {
    pub(crate) fn is_ready(&self) -> bool {
        self.errors.is_empty() && self.gaps.is_empty()
    }

    pub(super) fn error(&mut self, message: impl Into<String>) {
        if !self.quiet {
            self.errors.insert(message.into());
        }
    }

    pub(super) fn warning(&mut self, message: impl Into<String>) {
        if !self.quiet {
            self.warnings.insert(message.into());
        }
    }

    pub(super) fn note(&mut self, message: impl Into<String>) {
        if !self.quiet {
            self.notes.insert(message.into());
        }
    }

    pub(super) fn capability(&mut self, key: CapabilityKey) {
        self.capabilities.insert(key);
    }

    pub(super) fn gap(&mut self, key: CapabilityKey, reason: &'static str) {
        self.capabilities.insert(key.clone());
        self.gaps.entry(key).or_default().insert(reason);
    }

    pub(super) fn gap_at(&mut self, key: CapabilityKey, reason: &'static str, path: String) {
        self.gap(key.clone(), reason);
        self.gap_paths.entry(key).or_default().insert(path);
    }

    pub(super) fn explain(&mut self, message: impl Into<String>) {
        if self.explain {
            let message = message.into();
            if !self.explanations.contains(&message) {
                self.explanations.push(message);
            }
        }
    }

    pub(crate) fn print(&self) {
        for error in &self.errors {
            println!("ERROR {error}");
        }
        for (key, reasons) in &self.gaps {
            let reasons = reasons.iter().copied().collect::<Vec<_>>().join(", ");
            println!(
                "GAP   {} opcode={} type={} reasons={reasons}",
                key.domain, key.opcode, key.type_name
            );
        }
        for warning in &self.warnings {
            println!("WARN  {warning}");
        }
        for note in &self.notes {
            println!("NOTE  {note}");
        }
        for explanation in &self.explanations {
            println!("INFO  {explanation}");
        }
        println!(
            "{} skills={} buffs={} errors={} warnings={} notes={} gaps={}",
            if self.is_ready() {
                "READY"
            } else {
                "INCOMPLETE"
            },
            self.checked_skills.len(),
            self.checked_buffs.len(),
            self.errors.len(),
            self.warnings.len(),
            self.notes.len(),
            self.gaps.len(),
        );
    }

    pub(crate) fn json(&self) -> serde_json::Value {
        let mut skills = self.checked_skills.iter().copied().collect::<Vec<_>>();
        let mut buffs = self.checked_buffs.iter().copied().collect::<Vec<_>>();
        skills.sort_unstable();
        buffs.sort_unstable();
        let capabilities = self
            .capabilities
            .iter()
            .map(|key| {
                let reasons = self.gaps.get(key).cloned().unwrap_or_default();
                serde_json::json!({
                    "domain": key.domain,
                    "opcode": key.opcode,
                    "type": key.type_name,
                    "semantic": if reasons.is_empty() { "supported" } else { "unsupported" },
                    "reasons": reasons,
                    "paths": self.gap_paths.get(key).cloned().unwrap_or_default(),
                })
            })
            .collect::<Vec<_>>();
        serde_json::json!({
            "ready": self.is_ready(),
            "skills": skills,
            "buffs": buffs,
            "capabilities": capabilities,
            "errors": self.errors,
            "warnings": self.warnings,
            "notes": self.notes,
            "explanations": self.explanations,
        })
    }

    pub(crate) fn print_json(&self) {
        println!(
            "{}",
            serde_json::to_string_pretty(&self.json()).expect("support report serializes")
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_distinguishes_supported_and_unsupported_capabilities() {
        let supported = CapabilityKey::new("condition", 1, "Ready");
        let unsupported = CapabilityKey::new("behavior", 2, "MissingOwner");
        let mut report = Report::default();
        report.capability(supported);
        report.gap(unsupported, "missing command owner");
        report.explanations.push("exact route".to_owned());

        let json = report.json();
        assert!(!json["ready"].as_bool().unwrap());
        assert_eq!(json["capabilities"][0]["semantic"], "unsupported");
        assert_eq!(json["capabilities"][1]["semantic"], "supported");
        assert_eq!(json["explanations"][0], "exact route");
    }
}
