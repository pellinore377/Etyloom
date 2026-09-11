use crate::{
    content,
    phonology::{self, Law},
    random::Random,
};
use etyloom_core::*;
use std::collections::{BTreeMap, BTreeSet};

pub fn generate(recipe: Recipe) -> Result<Package> {
    generate_with(recipe, |_| Ok(()), || false)
}

pub fn generate_with(
    recipe: Recipe,
    mut progress: impl FnMut(&str) -> Result<()>,
    canceled: impl Fn() -> bool,
) -> Result<Package> {
    recipe.validate()?;
    progress("Selecting compatible grammatical mechanisms")?;
    let sounds = phonology::inventory(&recipe)?;
    let mut grammar = grammar(&recipe, &sounds)?;
    let mut entries = ancestor(&recipe, &sounds, &grammar)?;
    let mut stages = vec![Stage {
        index: 0,
        name: "Ancestral".into(),
        grammar: grammar.clone(),
        events: vec![],
    }];
    progress("Ancestral vocabulary and paradigms are ready")?;
    for stage in 1..=recipe.history_depth {
        if canceled() {
            return Err(Error::Canceled);
        }
        let law = match stage {
            1 => Law::IntervocalicVoicing,
            2 if grammar.morphology != Morphology::Analytic => Law::IFronting,
            3 if grammar.morphology != Morphology::Analytic => Law::FinalILoss,
            2 | 5 | 7 => Law::FinalDevoicing,
            _ => Law::Spirantization,
        };
        let mut affected = 0;
        for entry in &mut entries {
            let previous = entry.current()?.clone();
            let form = Paradigm {
                stage,
                base: phonology::apply(&previous.base, law),
                plural: previous.plural.as_ref().map(|f| phonology::apply(f, law)),
                past: previous.past.as_ref().map(|f| phonology::apply(f, law)),
            };
            if form.base != previous.base
                || form.plural != previous.plural
                || form.past != previous.past
            {
                affected += 1;
            }
            entry.forms.push(form);
        }
        for form in grammar
            .markers
            .values_mut()
            .chain(grammar.pronouns.values_mut())
        {
            let changed = phonology::apply(form, law);
            if !changed.0.is_empty() {
                *form = changed;
            }
        }
        let (id, title, description) = phonology::describe(law);
        let mut events = vec![Event {
            id: id.into(),
            title: title.into(),
            description: description.into(),
            affected,
        }];
        if stage == 4 && grammar.morphology != Morphology::Analytic {
            let suffix = marker(&grammar, "renewed_plural")?.clone();
            grammar.markers.insert("plural".into(), suffix.clone());
            let mut count = 0;
            for entry in entries.iter_mut().filter(|e| e.category == Category::Noun) {
                let mut rng = Random::new(&recipe, &format!("analogy/{}", entry.id));
                let threshold = if grammar.morphology == Morphology::Mixed {
                    1500
                } else {
                    6000
                };
                if entry.frequency < 4000 && rng.below(10000)? < threshold {
                    if let Some(current) = entry.forms.last_mut() {
                        current.plural = Some(current.base.joined(&suffix));
                        count += 1;
                    }
                }
            }
            events.push(Event {
                id: "analogical-renewal".into(), title: "A new plural pattern spreads".into(),
                description: "A renewed collective ending becomes productive. Lower-frequency nouns preferentially adopt it; older, frequent paradigms retain their inherited alternations.".into(), affected: count,
            });
        }
        stages.push(Stage {
            index: stage,
            name: if stage == recipe.history_depth {
                "Contemporary".into()
            } else {
                format!("Period {stage}")
            },
            grammar: grammar.clone(),
            events,
        });
        progress(&format!(
            "Applied historical stage {stage} of {}",
            recipe.history_depth
        ))?;
    }
    if canceled() {
        return Err(Error::Canceled);
    }
    progress("Coining contemporary compounds and compiling the lexicon")?;
    let added = expand(&recipe, &grammar, &mut entries)?;
    if let Some(stage) = stages.last_mut() {
        stage.events.push(Event {
            id: "lexical-expansion".into(), title: "New words enter the language".into(),
            description: "Transparent noun compounds are coined from contemporary stems. They follow current inflection and do not undergo sound changes that predate their creation. These are generated coinages, not corpus-attested usages.".into(), affected: added,
        });
    }
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    let inventory: BTreeSet<_> = entries
        .iter()
        .filter_map(|e| e.forms.last())
        .flat_map(|p| p.base.0.iter().copied())
        .collect();
    let mut package = Package {
        schema: SCHEMA, revision: String::new(), recipe, inventory: inventory.into_iter().collect(), grammar, stages, lexicon: entries, examples: vec![],
        capabilities: ["seeded generation", "six constituent orders", "particle and suffix morphology", "historical stem alternation", "frequency-biased analogy", "dated lexical innovation", "controlled bidirectional translation", "scope-aware negation and modality", "polar questions", "clause coordination", "adjectives and plural noun phrases", "revision-bound exercises", "parallel corpus export"].into_iter().map(String::from).collect(),
        limitations: [
            "This release implements a controlled grammatical fragment, not unrestricted translation.",
            "Supported meanings: simple present, past and future events, one adjective per noun phrase, number, same-subject want/can/must, negation, polar questions and clause coordination.",
            "Relative clauses, different-subject complements, possession, aspect, comparison, tone, stress-conditioned changes, arbitrary sound rules and neural inference are not implemented yet.",
            "The lexicon contains original editorial roots and transparently labeled generated compounds; it is not a 5,000-root attested dictionary.",
            "Historical laws and generation weights are editorial mechanisms, not empirical predictions. Community notes are stored but not automatically interpreted.",
            "No phonetic audio or pronunciation assessment is supplied. IPA is segmental; prosodic annotation is pending.",
        ].into_iter().map(String::from).collect(),
    };
    progress("Validating grammar, histories and example sentences")?;
    package.examples = crate::corpus::examples(&package, 96)?;
    crate::validate(&package)?;
    package.revision = fingerprint(&package)?;
    progress("Validated language is ready to save")?;
    Ok(package)
}

pub fn marker<'a>(grammar: &'a Grammar, name: &str) -> Result<&'a Form> {
    grammar
        .markers
        .get(name)
        .ok_or_else(|| Error::Package(format!("Missing marker {name}")))
}

fn grammar(recipe: &Recipe, sounds: &[Sound]) -> Result<Grammar> {
    let mut rng = Random::new(recipe, "grammar/selection");
    let morphology = match recipe.morphology {
        Some(value) => value,
        None => rng.weighted(&[
            (Morphology::Analytic, 4),
            (Morphology::Suffixing, 4),
            (Morphology::Mixed, 3),
        ])?,
    };
    let order = match recipe.order {
        Some(value) => value,
        None => rng.weighted(&[
            (Order::Svo, 4),
            (Order::Sov, 4),
            (Order::Vso, 2),
            (Order::Vos, 1),
            (Order::Ovs, 1),
            (Order::Osv, 1),
        ])?,
    };
    let mut used = BTreeSet::new();
    let mut markers = BTreeMap::new();
    for key in [
        "plural",
        "renewed_plural",
        "past",
        "future",
        "object",
        "not",
        "want",
        "can",
        "must",
        "question",
        "and",
    ] {
        let value = unique_marker(recipe, sounds, key, &mut used)?;
        markers.insert(key.into(), value);
    }
    if morphology != Morphology::Analytic {
        markers.insert("plural".into(), Form(vec![Sound::I]));
    }
    let mut pronouns = BTreeMap::new();
    for key in ["1s", "2s", "3s", "1p", "2p", "3p"] {
        pronouns.insert(key.into(), unique_marker(recipe, sounds, key, &mut used)?);
    }
    Ok(Grammar {
        morphology,
        order,
        adjective_before: rng.below(2)? == 0,
        case_marking: morphology != Morphology::Analytic && rng.below(4)? != 0,
        negation_after: rng.below(2)? == 0,
        markers,
        pronouns,
    })
}

fn unique_marker(
    recipe: &Recipe,
    sounds: &[Sound],
    key: &str,
    used: &mut BTreeSet<String>,
) -> Result<Form> {
    for attempt in 0..128 {
        let mut form = phonology::word(recipe, sounds, &format!("marker/{key}/{attempt}"), true)?;
        form.0.push(Sound::L);
        if used.insert(form.text()) {
            return Ok(form);
        }
    }
    Err(Error::Budget)
}

fn ancestor(recipe: &Recipe, sounds: &[Sound], grammar: &Grammar) -> Result<Vec<Entry>> {
    content::concepts()?
        .into_iter()
        .take(recipe.lexicon_size)
        .map(|concept| {
            let base = phonology::word(recipe, sounds, &format!("lexeme/{}", concept.id), false)?;
            let paradigm = inflect(base, concept.category, grammar, 0)?;
            Ok(Entry {
                id: concept.id,
                gloss: concept.english.base.clone(),
                category: concept.category,
                transitive: concept.transitive,
                frequency: concept.frequency,
                english: concept.english,
                origin: Origin::Root,
                introduced: 0,
                forms: vec![paradigm],
            })
        })
        .collect()
}

pub fn inflect(
    base: Form,
    category: Category,
    grammar: &Grammar,
    stage: usize,
) -> Result<Paradigm> {
    let plural = if category == Category::Noun && grammar.morphology != Morphology::Analytic {
        Some(base.joined(marker(grammar, "plural")?))
    } else {
        None
    };
    let past = if category == Category::Verb && grammar.morphology != Morphology::Analytic {
        Some(base.joined(marker(grammar, "past")?))
    } else {
        None
    };
    Ok(Paradigm {
        stage,
        base,
        plural,
        past,
    })
}

fn expand(recipe: &Recipe, grammar: &Grammar, entries: &mut Vec<Entry>) -> Result<usize> {
    let needed = recipe.lexicon_size.saturating_sub(entries.len());
    if needed == 0 {
        return Ok(0);
    }
    let nouns: Vec<_> = entries
        .iter()
        .filter(|e| e.category == Category::Noun)
        .cloned()
        .collect();
    let mut candidates = Vec::new();
    for modifier in &nouns {
        for head in &nouns {
            if modifier.id == head.id {
                continue;
            }
            let id = format!("compound/{}/{}", modifier.id, head.id);
            let mut rng = Random::new(recipe, &id);
            candidates.push((rng.below(usize::MAX / 2)?, id, modifier, head));
        }
    }
    candidates.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
    if candidates.len() < needed {
        return Err(Error::Invalid(
            "The content pack cannot supply the requested vocabulary size".into(),
        ));
    }
    for (_, id, modifier, head) in candidates.into_iter().take(needed) {
        let base = if grammar.adjective_before {
            modifier.current()?.base.joined(&head.current()?.base)
        } else {
            head.current()?.base.joined(&modifier.current()?.base)
        };
        let gloss = format!("{} {}", modifier.gloss, head.gloss);
        entries.push(Entry {
            id,
            gloss: gloss.clone(),
            category: Category::Noun,
            transitive: false,
            frequency: 100,
            english: English {
                base: gloss,
                plural: format!("{} {}", modifier.english.base, head.english.plural),
                past: String::new(),
                third: String::new(),
            },
            origin: Origin::Compound {
                modifier: modifier.id.clone(),
                head: head.id.clone(),
            },
            introduced: recipe.history_depth,
            forms: vec![inflect(
                base,
                Category::Noun,
                grammar,
                recipe.history_depth,
            )?],
        });
    }
    Ok(needed)
}

pub fn fingerprint(package: &Package) -> Result<String> {
    let mut canonical = package.clone();
    canonical.revision.clear();
    Ok(blake3::hash(&serde_json::to_vec(&canonical)?)
        .to_hex()
        .to_string())
}
