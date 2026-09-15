use crate::{
    content, corpus,
    generate::{fingerprint, marker},
    history, phonology, phonotactics,
    random::Random,
};
use etyloom_core::*;
use std::collections::{BTreeMap, BTreeSet};

pub fn generate_with(
    recipe: Recipe,
    mut progress: impl FnMut(&str) -> Result<()>,
    canceled: impl Fn() -> bool,
) -> Result<Package> {
    recipe.validate()?;
    if canceled() {
        return Err(Error::Canceled);
    }
    progress("Selecting grammar and syllable constraints")?;
    let sounds = phonology::inventory(&recipe)?;
    let mut grammar = grammar(&recipe, &sounds)?;
    let mut entries = ancestor(&recipe, &sounds, &grammar)?;
    let schedule = schedule(&recipe, &entries)?;
    let introduced = introduce(&schedule, &grammar, &mut entries, 0)?;
    let mut stages = vec![Stage {
        index: 0,
        name: "Ancestral".into(),
        grammar: grammar.clone(),
        events: vec![birth_event(entries.len())],
        metrics: Some(StageMetrics {
            inherited: 0,
            introduced: entries.len(),
            changed_headwords: 0,
            changed_paradigms: 0,
            changed_grammar_forms: 0,
        }),
    }];
    progress(&format!(
        "Ancestral lexicon ready, including {introduced} early compounds"
    ))?;
    let mut used = BTreeSet::new();
    for stage in 1..=recipe.history_depth {
        if canceled() {
            return Err(Error::Canceled);
        }
        let before: Vec<_> = entries
            .iter()
            .map(|e| e.current().cloned())
            .collect::<Result<_>>()?;
        let previous_grammar = grammar.clone();
        let mut events = Vec::new();
        if let Some((rule, next)) = history::select(&recipe, &grammar, &entries, &used, stage)? {
            let constraints = profile(&grammar)?;
            let mut affected = 0;
            let mut repaired = 0;
            for (index, entry) in entries.iter_mut().enumerate() {
                if index % 128 == 0 && canceled() {
                    return Err(Error::Canceled);
                }
                let prior = entry.current()?;
                let form = history::change_paradigm(rule, constraints, prior, stage)?;
                if history::changed(prior, &form) {
                    affected += 1;
                }
                if form.base != rule.apply(&prior.base) {
                    repaired += 1;
                }
                entry.forms.push(form);
            }
            let (id, title, description) = rule.description();
            used.insert(id.to_owned());
            events.push(Event { id: id.into(), title: title.into(), description: format!("{description} Only vocabulary already present participates. {repaired} headwords also required the language's epenthetic syllable repair."), affected });
            grammar = next;
        } else {
            for entry in &mut entries {
                let mut form = entry.current()?.clone();
                form.stage = stage;
                entry.forms.push(form);
            }
            events.push(Event { id: "stable-period".into(), title: "A period of phonological stability".into(), description: "No unused rule in the current catalog had an effect compatible with this grammar. No artificial changes were added.".into(), affected: 0 });
        }
        if used.contains("final-i-loss")
            && !used.contains("analogical-renewal")
            && grammar.morphology != Morphology::Analytic
            && let Some(count) = renew_plural(&recipe, &mut grammar, &mut entries)?
        {
            used.insert("analogical-renewal".into());
            events.push(Event {
                    id: "analogical-renewal".into(),
                    title: "A new plural pattern spreads".into(),
                    description: "A renewed collective ending becomes productive. Lower-frequency nouns preferentially adopt it; frequent nouns more often retain inherited alternations.".into(),
                    affected: count,
                });
        }
        let changed_headwords = before
            .iter()
            .zip(&entries)
            .filter(|(a, b)| b.forms.last().is_some_and(|f| f.base != a.base))
            .count();
        let changed_paradigms = before
            .iter()
            .zip(&entries)
            .filter(|(a, b)| b.forms.last().is_some_and(|f| history::changed(a, f)))
            .count();
        let changed_grammar_forms = grammar
            .markers
            .iter()
            .filter(|(k, v)| previous_grammar.markers.get(*k) != Some(*v))
            .count()
            + grammar
                .pronouns
                .iter()
                .filter(|(k, v)| previous_grammar.pronouns.get(*k) != Some(*v))
                .count();
        let introduced = introduce(&schedule, &grammar, &mut entries, stage)?;
        if introduced > 0 {
            events.push(birth_event(introduced));
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
            metrics: Some(StageMetrics {
                inherited: before.len(),
                introduced,
                changed_headwords,
                changed_paradigms,
                changed_grammar_forms,
            }),
        });
        progress(&format!(
            "Stage {stage}/{}: {changed_paradigms} inherited entries changed; {introduced} introduced",
            recipe.history_depth
        ))?;
    }
    if canceled() {
        return Err(Error::Canceled);
    }
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    let inventory: BTreeSet<_> = entries
        .iter()
        .flat_map(|e| e.forms.last())
        .flat_map(|p| {
            std::iter::once(&p.base)
                .chain(p.plural.iter())
                .chain(p.past.iter())
                .chain(p.future.iter())
        })
        .chain(grammar.markers.values())
        .chain(grammar.pronouns.values())
        .flat_map(|f| f.0.iter().copied())
        .collect();
    let mut package = Package {
        schema: SCHEMA, revision: String::new(), recipe, inventory: inventory.into_iter().collect(), grammar, stages,
        lexicon: entries, examples: vec![],
        capabilities: ["seeded generation", "versioned legacy replay", "language-specific syllable constraints", "boundary-aware compounds and inflection", "staged lexical introduction", "effect-tested historical rules", "historical stem alternation", "frequency-biased analogy", "six constituent orders", "controlled bidirectional translation", "scope-aware negation and modality", "polar questions", "clause coordination", "revision-bound exercises", "parallel corpus export"].map(String::from).to_vec(),
        limitations: [
            "This release implements a controlled grammatical fragment, not unrestricted translation.",
            "The concept pack has 210 editorial root meanings. Larger lexicons contain explicitly labeled noun compounds, not thousands of independent curated senses.",
            "The bounded historical rule catalog is editorial, not an empirical prediction. Stable periods are reported when no suitable unused rule remains.",
            "The syllable profile is fixed across stages; changing phonotactic systems, rich borrowing and semantic evolution remain future work.",
            "Relative clauses, different-subject complements, possession, aspect, comparison, tone, arbitrary sound rules and neural inference are not yet implemented.",
            "World notes and the speech-community name are preserved metadata, not automatically interpreted linguistic instructions.",
        ].map(String::from).to_vec(),
    };
    progress("Validating histories and grammar")?;
    package.examples = corpus::examples(&package, 96)?;
    crate::validate(&package)?;
    package.revision = fingerprint(&package)?;
    Ok(package)
}

fn birth_event(count: usize) -> Event {
    Event { id: "lexical-introduction".into(), title: "Vocabulary enters the language".into(), description: "New forms are coined using this stage's stems and grammar. They undergo only subsequent sound changes.".into(), affected: count }
}

pub fn profile(grammar: &Grammar) -> Result<&Phonotactics> {
    grammar
        .phonotactics
        .as_ref()
        .ok_or_else(|| Error::Package("Missing syllable profile".into()))
}

fn grammar(recipe: &Recipe, sounds: &[Sound]) -> Result<Grammar> {
    let mut grammar = crate::generate::grammar(recipe, sounds)?;
    let profile = phonotactics::profile(recipe)?;
    grammar.phonotactics = Some(profile.clone());
    let mut used = BTreeSet::new();
    if grammar.morphology != Morphology::Analytic {
        used.insert("i".to_owned());
    }
    for (key, form) in grammar
        .markers
        .iter_mut()
        .chain(grammar.pronouns.iter_mut())
    {
        if key == "plural" && grammar.morphology != Morphology::Analytic {
            *form = Form(vec![Sound::I]);
            used.insert(form.text());
            continue;
        }
        let mut selected = None;
        for attempt in 0..256 {
            let address = format!("grammar/v2/{key}/{attempt}");
            let mut candidate =
                phonotactics::word(recipe, sounds, &profile, &address, attempt < 128)?;
            if key == "object" && !candidate.0.first().is_some_and(|s| s.vowel()) {
                candidate.0.insert(0, profile.linker);
            }
            if used.insert(candidate.text()) {
                selected = Some(candidate);
                break;
            }
        }
        *form = selected.ok_or(Error::Budget)?;
    }
    if !history::grammar_usable(&grammar) {
        return Err(Error::Package("Conflicting grammatical forms".into()));
    }
    Ok(grammar)
}

fn ancestor(recipe: &Recipe, sounds: &[Sound], grammar: &Grammar) -> Result<Vec<Entry>> {
    content::concepts()?
        .into_iter()
        .take(recipe.lexicon_size)
        .map(|concept| {
            let base = phonotactics::word(
                recipe,
                sounds,
                profile(grammar)?,
                &format!("lexeme/{}", concept.id),
                false,
            )?;
            Ok(Entry {
                id: concept.id,
                gloss: concept.english.base.clone(),
                category: concept.category,
                transitive: concept.transitive,
                frequency: concept.frequency,
                english: concept.english,
                origin: Origin::Root,
                introduced: 0,
                forms: vec![inflect(base, concept.category, grammar, 0)?],
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
    let mut form = crate::generate::inflect(base, category, grammar, stage)?;
    let constraints = profile(grammar)?;
    for value in std::iter::once(&mut form.base)
        .chain(form.plural.iter_mut())
        .chain(form.past.iter_mut())
        .chain(form.future.iter_mut())
    {
        *value = phonotactics::repair(constraints, value)?;
    }
    Ok(form)
}

struct Coinage {
    id: String,
    modifier: String,
    head: String,
    stage: usize,
}

fn schedule(recipe: &Recipe, entries: &[Entry]) -> Result<Vec<Coinage>> {
    let needed = recipe.lexicon_size.saturating_sub(entries.len());
    if needed == 0 {
        return Ok(vec![]);
    }
    let nouns: Vec<_> = entries
        .iter()
        .filter(|e| e.category == Category::Noun)
        .collect();
    let weights: Vec<_> = (0..=recipe.history_depth)
        .map(|stage| (stage, if stage == 0 { 4 } else { 1 }))
        .collect();
    let mut candidates = Vec::new();
    for modifier in &nouns {
        for head in &nouns {
            if modifier.id == head.id {
                continue;
            }
            let id = format!("compound/{}/{}", modifier.id, head.id);
            let priority = Random::new(recipe, &id).below(1_000_000_007)?;
            let stage = Random::new(recipe, &format!("birth/{id}")).weighted(&weights)?;
            candidates.push((
                priority,
                Coinage {
                    id,
                    modifier: modifier.id.clone(),
                    head: head.id.clone(),
                    stage,
                },
            ));
        }
    }
    candidates.sort_by(|(a, x), (b, y)| (a, &x.id).cmp(&(b, &y.id)));
    if candidates.len() < needed {
        return Err(Error::Invalid(
            "The content pack cannot supply the requested vocabulary size".into(),
        ));
    }
    Ok(candidates
        .into_iter()
        .take(needed)
        .map(|(_, c)| c)
        .collect())
}

fn introduce(
    schedule: &[Coinage],
    grammar: &Grammar,
    entries: &mut Vec<Entry>,
    stage: usize,
) -> Result<usize> {
    let roots: BTreeMap<_, _> = entries
        .iter()
        .filter(|e| matches!(e.origin, Origin::Root))
        .map(|e| (e.id.clone(), e.clone()))
        .collect();
    let mut count = 0;
    for coinage in schedule.iter().filter(|c| c.stage == stage) {
        let modifier = roots
            .get(&coinage.modifier)
            .ok_or_else(|| Error::Package("Missing compound modifier".into()))?;
        let head = roots
            .get(&coinage.head)
            .ok_or_else(|| Error::Package("Missing compound head".into()))?;
        let (left, right) = if grammar.adjective_before {
            (modifier, head)
        } else {
            (head, modifier)
        };
        let base = phonotactics::join(
            profile(grammar)?,
            &left.current()?.base,
            &right.current()?.base,
        )?;
        let gloss = format!("{} {}", modifier.gloss, head.gloss);
        entries.push(Entry {
            id: coinage.id.clone(),
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
            introduced: stage,
            forms: vec![inflect(base, Category::Noun, grammar, stage)?],
        });
        count += 1;
    }
    Ok(count)
}

fn renew_plural(
    recipe: &Recipe,
    grammar: &mut Grammar,
    entries: &mut [Entry],
) -> Result<Option<usize>> {
    let suffix = marker(grammar, "renewed_plural")?.clone();
    let mut trial = grammar.clone();
    trial.markers.insert("plural".into(), suffix.clone());
    if !history::grammar_usable(&trial) {
        return Ok(None);
    }
    *grammar = trial;
    let mut count = 0;
    for entry in entries.iter_mut().filter(|e| e.category == Category::Noun) {
        let threshold = if grammar.morphology == Morphology::Mixed {
            1500
        } else {
            6000
        };
        if entry.frequency >= 4000
            || Random::new(recipe, &format!("analogy/{}", entry.id)).below(10000)? >= threshold
        {
            continue;
        }
        let current = entry
            .forms
            .last_mut()
            .ok_or_else(|| Error::Package("Missing paradigm".into()))?;
        let renewed = phonotactics::join(profile(grammar)?, &current.base, &suffix)?;
        if current.plural.as_ref() != Some(&renewed) {
            current.plural = Some(renewed);
            count += 1;
        }
    }
    Ok(Some(count))
}
