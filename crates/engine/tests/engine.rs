use etyloom_core::*;
use etyloom_engine::{
    Runtime, generate,
    phonology::{self, Law},
    verify_import,
};

fn recipe(seed: &str) -> Recipe {
    Recipe {
        seed: seed.into(),
        lexicon_size: 128,
        ..Recipe::default()
    }
}

#[test]
fn reproducible_package_and_recipe() -> Result<()> {
    let a = generate(recipe("woven"))?;
    let b = generate(recipe("woven"))?;
    assert_eq!(serde_json::to_vec(&a)?, serde_json::to_vec(&b)?);
    assert_ne!(a.revision, generate(recipe("another"))?.revision);
    verify_import(&a)
}

#[test]
fn historical_alternation_is_executable() {
    use Sound::*;
    let plural = Form(vec![T, A, K, I]);
    let fronted = phonology::apply(&plural, Law::IFronting);
    assert_eq!(fronted, Form(vec![T, Ae, K, I]));
    assert_eq!(
        phonology::apply(&fronted, Law::FinalILoss),
        Form(vec![T, Ae, K])
    );
    assert_eq!(
        phonology::apply(&Form(vec![N, I]), Law::FinalILoss),
        Form(vec![N, I])
    );
}

#[test]
fn all_orders_and_morphologies_round_trip() -> Result<()> {
    for order in [
        Order::Svo,
        Order::Sov,
        Order::Vso,
        Order::Vos,
        Order::Ovs,
        Order::Osv,
    ] {
        for morphology in [
            Morphology::Analytic,
            Morphology::Suffixing,
            Morphology::Mixed,
        ] {
            let package = generate(Recipe {
                order: Some(order),
                morphology: Some(morphology),
                ..recipe("many-grammars")
            })?;
            let runtime = Runtime::new(&package)?;
            for sentence in [
                "I see the river",
                "we walked",
                "I want to not walk",
                "I do not want to walk",
                "the small child sees the old tree",
                "I will see the rivers",
                "I see you and you see me",
            ] {
                for meaning in runtime.interpret(sentence)? {
                    let rendered = runtime.realize(&meaning)?;
                    assert!(
                        runtime.analyze(&rendered)?.contains(&meaning),
                        "{order} {morphology:?}: {sentence}"
                    );
                }
            }
        }
    }
    Ok(())
}

#[test]
fn negation_scope_is_not_collapsed() -> Result<()> {
    let package = generate(recipe("scope"))?;
    let runtime = Runtime::new(&package)?;
    let a = runtime.interpret("I do not want to walk")?;
    let b = runtime.interpret("I want to not walk")?;
    assert_ne!(a, b);
    let a = runtime.realize(a.first().ok_or(Error::Budget)?)?;
    let b = runtime.realize(b.first().ok_or(Error::Budget)?)?;
    assert_ne!(a, b);
    Ok(())
}

#[test]
fn new_coinages_do_not_inherit_ancient_sound_laws() -> Result<()> {
    let package = generate(Recipe {
        lexicon_size: 512,
        ..recipe("late-words")
    })?;
    let late = package
        .lexicon
        .iter()
        .find(|e| matches!(e.origin, Origin::Compound { .. }))
        .ok_or(Error::Budget)?;
    assert_eq!(late.introduced, package.recipe.history_depth);
    assert_eq!(late.forms.len(), 1);
    assert_eq!(late.current()?.stage, package.recipe.history_depth);
    Ok(())
}

#[test]
fn unknown_input_never_becomes_unchanged_success() -> Result<()> {
    let package = generate(recipe("strict"))?;
    let runtime = Runtime::new(&package)?;
    assert!(runtime.translate("I teleport a spaceship", false).is_err());
    assert!(runtime.translate("unknown unknown unknown", true).is_err());
    assert!(runtime.interpret("I want you to walk").is_err());
    Ok(())
}

#[test]
fn mutation_breaks_package_checksum() -> Result<()> {
    let mut package = generate(recipe("immutable"))?;
    package.recipe.name = "changed".into();
    assert!(verify_import(&package).is_err());
    Ok(())
}

#[test]
fn recipe_budgets_are_enforced() {
    assert!(
        generate(Recipe {
            lexicon_size: usize::MAX,
            ..recipe("too-big")
        })
        .is_err()
    );
    assert!(
        generate(Recipe {
            engine: "future/9".into(),
            ..recipe("future")
        })
        .is_err()
    );
}

#[test]
fn cancellation_is_explicit() {
    let result = etyloom_engine::generate_with(recipe("cancel"), |_| Ok(()), || true);
    assert!(matches!(result, Err(Error::Canceled)));
}

#[test]
fn corpus_is_revision_bound_and_bidirectional() -> Result<()> {
    let package = generate(recipe("corpus"))?;
    let mut bytes = Vec::new();
    etyloom_engine::corpus::write_corpus(&package, 16, &mut bytes)?;
    let text = std::str::from_utf8(&bytes).map_err(|e| Error::Invalid(e.to_string()))?;
    assert_eq!(text.lines().count(), 32);
    for line in text.lines() {
        let row: serde_json::Value = serde_json::from_str(line)?;
        assert_eq!(row["revision"], package.revision);
        assert!(row["source"].as_str().is_some_and(|v| !v.is_empty()));
    }
    Ok(())
}

proptest::proptest! {
    #[test]
    fn sound_laws_preserve_nonempty_minimal_forms(sounds in proptest::collection::vec(proptest::sample::select(vec![Sound::P, Sound::T, Sound::A, Sound::I]), 1..32)) {
        let form = Form(sounds);
        let output = phonology::apply(&form, Law::FinalILoss);
        proptest::prop_assert!(!output.0.is_empty());
    }
}
