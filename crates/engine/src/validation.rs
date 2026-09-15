use crate::{evolution, history, phonotactics};
use etyloom_core::*;
use std::collections::{BTreeMap, BTreeSet};

pub fn development(package: &Package) -> Result<()> {
    if package.recipe.engine == LEGACY_ENGINE {
        if package.grammar.phonotactics.is_some()
            || package
                .stages
                .iter()
                .any(|s| s.metrics.is_some() || s.grammar.phonotactics.is_some())
        {
            return Err(Error::Package(
                "Legacy packages cannot contain newer generation fields".into(),
            ));
        }
        return Ok(());
    }
    if package.stages.last().map(|s| &s.grammar) != Some(&package.grammar) {
        return Err(Error::Package(
            "Contemporary grammar does not match the final stage".into(),
        ));
    }
    let births: BTreeMap<_, _> = package
        .lexicon
        .iter()
        .map(|e| (e.id.as_str(), e.introduced))
        .collect();
    let mut used_rules = BTreeSet::new();
    for (index, stage) in package.stages.iter().enumerate() {
        if stage.index != index {
            return Err(Error::Package("Unordered historical stages".into()));
        }
        let profile = evolution::profile(&stage.grammar)?;
        phonotactics::validate(profile)?;
        if !history::grammar_usable(&stage.grammar) {
            return Err(Error::Package("Collapsed grammatical control forms".into()));
        }
        for form in stage
            .grammar
            .markers
            .values()
            .chain(stage.grammar.pronouns.values())
        {
            if !phonotactics::accepts(profile, form) {
                return Err(Error::Package(
                    "A grammatical form violates its syllable profile".into(),
                ));
            }
        }
        for event in &stage.events {
            if history::Rule::all()
                .iter()
                .any(|r| r.description().0 == event.id)
                && !used_rules.insert(&event.id)
            {
                return Err(Error::Package(
                    "Historical sound rules cannot be replayed as empty filler".into(),
                ));
            }
        }
        let mut counts = StageMetrics {
            inherited: 0,
            introduced: 0,
            changed_headwords: 0,
            changed_paradigms: 0,
            changed_grammar_forms: 0,
        };
        for entry in &package.lexicon {
            if entry.introduced > index {
                continue;
            }
            let offset = index - entry.introduced;
            let form = entry
                .forms
                .get(offset)
                .ok_or_else(|| Error::Package("Missing stage paradigm".into()))?;
            if offset == 0 {
                counts.introduced += 1;
            } else {
                counts.inherited += 1;
                let before = &entry.forms[offset - 1];
                counts.changed_headwords += usize::from(form.base != before.base);
                counts.changed_paradigms += usize::from(history::changed(form, before));
            }
            for value in std::iter::once(&form.base)
                .chain(form.plural.iter())
                .chain(form.past.iter())
                .chain(form.future.iter())
            {
                if !phonotactics::accepts(profile, value) {
                    return Err(Error::Package(format!(
                        "{} violates the syllable profile at stage {index}",
                        entry.id
                    )));
                }
            }
            if offset == 0
                && let Origin::Compound { modifier, head } = &entry.origin
            {
                for dependency in [modifier, head] {
                    if births
                        .get(dependency.as_str())
                        .is_none_or(|&born| born > index)
                    {
                        return Err(Error::Package(
                            "A compound predates its source words".into(),
                        ));
                    }
                }
            }
        }
        if index > 0 {
            let previous = &package.stages[index - 1].grammar;
            counts.changed_grammar_forms = stage
                .grammar
                .markers
                .iter()
                .filter(|(k, v)| previous.markers.get(*k) != Some(*v))
                .count()
                + stage
                    .grammar
                    .pronouns
                    .iter()
                    .filter(|(k, v)| previous.pronouns.get(*k) != Some(*v))
                    .count();
        }
        if stage.metrics.as_ref() != Some(&counts) {
            return Err(Error::Package(
                "Stage change counts disagree with recorded forms".into(),
            ));
        }
    }
    let actual: BTreeSet<_> = package
        .lexicon
        .iter()
        .flat_map(|e| e.forms.last())
        .flat_map(|p| {
            std::iter::once(&p.base)
                .chain(p.plural.iter())
                .chain(p.past.iter())
                .chain(p.future.iter())
        })
        .chain(package.grammar.markers.values())
        .chain(package.grammar.pronouns.values())
        .flat_map(|f| f.0.iter().copied())
        .collect();
    if package.inventory != actual.into_iter().collect::<Vec<_>>() {
        return Err(Error::Package(
            "Inventory is incomplete or contains unused sounds".into(),
        ));
    }
    Ok(())
}
