use etyloom_core::*;
use etyloom_engine::{Runtime, fingerprint, generate, history, phonotactics, verify_import};
use std::collections::BTreeSet;

fn reported() -> Result<Recipe> {
    Ok(serde_json::from_str(include_str!("fixtures/roc-v1.json"))?)
}

fn updated() -> Result<Package> {
    generate(Recipe {
        engine: ENGINE.into(),
        ..reported()?
    })
}

#[test]
fn original_report_replays_without_changing_a_byte_of_its_canonical_data() -> Result<()> {
    let package = generate(reported()?)?;
    assert_eq!(
        package.revision,
        "08dcef8343f94d41f251452c9e0d805048b19802304689b0610b58506e39c133"
    );
    assert_eq!(
        package.lexicon.iter().filter(|e| e.introduced == 8).count(),
        4790
    );
    for stage in &package.stages[6..] {
        assert!(
            stage
                .events
                .iter()
                .filter(|e| e.id != "lexical-expansion")
                .all(|e| e.affected == 0)
        );
    }
    verify_import(&package)
}

#[test]
fn reported_recipe_no_longer_exhausts_its_history_after_period_five() -> Result<()> {
    let package = updated()?;
    for stage in &package.stages[6..] {
        let counts = stage.metrics.as_ref().ok_or(Error::Budget)?;
        assert!(counts.changed_paradigms > 0, "empty period {}", stage.index);
        assert!(counts.inherited > 3000);
    }
    assert!(package.lexicon.iter().filter(|e| e.introduced < 8).count() > 4000);
    assert_eq!(package.lexicon.len(), 5000);
    let again = updated()?;
    assert_eq!(serde_json::to_vec(&package)?, serde_json::to_vec(&again)?);
    verify_import(&package)
}

#[test]
fn whole_paradigms_and_grammatical_forms_obey_their_stage_profile() -> Result<()> {
    let package = updated()?;
    for stage in &package.stages {
        let constraints = stage.grammar.phonotactics.as_ref().ok_or(Error::Budget)?;
        for entry in &package.lexicon {
            for p in entry.forms.iter().filter(|p| p.stage == stage.index) {
                for f in std::iter::once(&p.base)
                    .chain(p.plural.iter())
                    .chain(p.past.iter())
                    .chain(p.future.iter())
                {
                    assert!(
                        phonotactics::accepts(constraints, f),
                        "{} /{}",
                        entry.gloss,
                        f.ipa()
                    );
                }
            }
        }
    }
    Ok(())
}

#[test]
fn stages_count_changed_entries_once_and_keep_births_separate() -> Result<()> {
    let package = updated()?;
    for stage in &package.stages[1..] {
        let mut changed = 0;
        for entry in &package.lexicon {
            for pair in entry.forms.windows(2).filter(|p| p[1].stage == stage.index) {
                changed += usize::from(history::changed(&pair[0], &pair[1]));
            }
        }
        assert_eq!(
            stage
                .metrics
                .as_ref()
                .ok_or(Error::Budget)?
                .changed_paradigms,
            changed
        );
    }
    let mut forged = package;
    forged.stages[2]
        .metrics
        .as_mut()
        .ok_or(Error::Budget)?
        .changed_paradigms += 1;
    forged.revision = fingerprint(&forged)?;
    assert!(verify_import(&forged).is_err());
    Ok(())
}

#[test]
fn new_compounds_use_the_grammar_and_stems_at_birth() -> Result<()> {
    let package = updated()?;
    let mut birth_stages = BTreeSet::new();
    for entry in &package.lexicon {
        let Origin::Compound { modifier, head } = &entry.origin else {
            continue;
        };
        birth_stages.insert(entry.introduced);
        assert_eq!(
            entry.forms.first().ok_or(Error::Budget)?.stage,
            entry.introduced
        );
        assert_eq!(entry.forms.len(), 9 - entry.introduced);
        let grammar = &package.stages[entry.introduced].grammar;
        let form = |id: &str| -> Result<&Form> {
            let source = package
                .lexicon
                .iter()
                .find(|e| e.id == id)
                .ok_or(Error::Budget)?;
            Ok(&source
                .forms
                .iter()
                .find(|p| p.stage == entry.introduced)
                .ok_or(Error::Budget)?
                .base)
        };
        let (a, b) = if grammar.adjective_before {
            (modifier, head)
        } else {
            (head, modifier)
        };
        let expected = phonotactics::join(
            grammar.phonotactics.as_ref().ok_or(Error::Budget)?,
            form(a)?,
            form(b)?,
        )?;
        assert_eq!(entry.forms[0].base, expected);
    }
    assert_eq!(birth_stages.len(), 9);
    Ok(())
}

#[test]
fn live_case_suffixes_remain_syllabifiable_and_parseable() -> Result<()> {
    let package = updated()?;
    let runtime = Runtime::new(&package)?;
    for entry in package
        .lexicon
        .iter()
        .filter(|e| e.category == Category::Noun)
        .step_by(37)
    {
        for plural in [false, true] {
            let meaning = Meaning::Event {
                subject: Entity::Pronoun {
                    person: 1,
                    plural: false,
                },
                verb: "v.see".into(),
                object: Some(Entity::Noun {
                    id: entry.id.clone(),
                    plural,
                    adjective: None,
                }),
                tense: Tense::Present,
            };
            let output = runtime.realize(&meaning)?;
            assert!(runtime.analyze(&output)?.contains(&meaning));
        }
    }
    Ok(())
}

#[test]
fn sample_different_seeds_profiles_and_histories() -> Result<()> {
    for index in 0..24 {
        let sound = [SoundStyle::Fluid, SoundStyle::Balanced, SoundStyle::Crisp][index % 3];
        let package = generate(Recipe {
            seed: format!("history-regression-{index}"),
            sound,
            lexicon_size: 512,
            history_depth: index % 8 + 1,
            ..Recipe::default()
        })?;
        verify_import(&package)?;
        let mut used = BTreeSet::new();
        for stage in package.stages {
            for event in stage.events {
                if history::Rule::all()
                    .iter()
                    .any(|r| r.description().0 == event.id)
                {
                    assert!(used.insert(event.id));
                }
            }
        }
    }
    Ok(())
}
