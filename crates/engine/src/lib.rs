#![forbid(unsafe_code)]

pub mod content;
pub mod corpus;
pub mod generate;
pub mod grammar;
pub mod phonology;
pub mod random;

pub use generate::{fingerprint, generate, generate_with};
pub use grammar::Runtime;
use etyloom_core::*;
use std::collections::BTreeSet;

pub fn validate(package: &Package) -> Result<()> {
    package.recipe.validate()?;
    if package.schema != SCHEMA || package.stages.len() != package.recipe.history_depth + 1 || package.lexicon.len() != package.recipe.lexicon_size {
        return Err(Error::Package("Manifest counts do not match their contents".into()));
    }
    let mut ids = BTreeSet::new();
    for entry in &package.lexicon {
        if !ids.insert(&entry.id) { return Err(Error::Package(format!("Duplicate entry {}", entry.id))); }
        if entry.introduced > package.recipe.history_depth || entry.forms.len() != package.recipe.history_depth + 1 - entry.introduced {
            return Err(Error::Package(format!("Incomplete history for {}", entry.id)));
        }
        for (offset, paradigm) in entry.forms.iter().enumerate() {
            if paradigm.stage != entry.introduced + offset { return Err(Error::Package("Unordered historical forms".into())); }
            for form in std::iter::once(&paradigm.base).chain(paradigm.plural.iter()).chain(paradigm.past.iter()) {
                if form.0.is_empty() || form.0.len() > 128 || !form.0.iter().any(|s| s.vowel()) {
                    return Err(Error::Package(format!("Invalid phonological form in {}", entry.id)));
                }
            }
        }
    }
    for stage in &package.stages {
        for key in ["plural", "renewed_plural", "past", "future", "object", "not", "want", "can", "must", "question", "and"] {
            if generate::marker(&stage.grammar, key)?.0.is_empty() { return Err(Error::Package(format!("Empty grammatical marker {key}"))); }
        }
    }
    let runtime = Runtime::new(package)?;
    for example in &package.examples {
        let sentence = runtime.realize(&example.meaning)?;
        if sentence != example.text || !runtime.analyze(&sentence)?.contains(&example.meaning) {
            return Err(Error::Package("An example does not round-trip through the grammar".into()));
        }
    }
    Ok(())
}

pub fn verify_import(package: &Package) -> Result<()> {
    validate(package)?;
    if package.revision != fingerprint(package)? { return Err(Error::Package("Revision checksum does not match the package".into())); }
    Ok(())
}
