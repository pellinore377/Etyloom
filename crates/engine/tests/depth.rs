use etyloom_core::*;
use etyloom_engine::{Runtime, generate};

#[test]
fn english_and_conlang_preserve_nested_modal_meanings() -> Result<()> {
    let language = generate(Recipe {
        seed: "nested-modals".into(),
        lexicon_size: 128,
        ..Recipe::default()
    })?;
    let runtime = Runtime::new(&language)?;
    for english in [
        "I cannot walk",
        "I do not have to walk",
        "I want to be able to walk",
        "I can not walk",
        "I want to have to walk",
    ] {
        for meaning in runtime.interpret(english)? {
            let generated = runtime.english(&meaning)?;
            assert!(
                runtime.interpret(&generated)?.contains(&meaning),
                "{english} -> {generated}"
            );
            assert!(
                runtime
                    .analyze(&runtime.realize(&meaning)?)?
                    .contains(&meaning)
            );
        }
    }
    assert_ne!(
        runtime.interpret("I cannot walk")?,
        runtime.interpret("I can not walk")?
    );
    Ok(())
}

#[test]
fn future_paradigms_have_complete_history() -> Result<()> {
    let language = generate(Recipe {
        morphology: Some(Morphology::Suffixing),
        lexicon_size: 128,
        ..Recipe::default()
    })?;
    for entry in language
        .lexicon
        .iter()
        .filter(|entry| entry.category == Category::Verb)
    {
        assert!(entry.forms.iter().all(|paradigm| paradigm.future.is_some()));
    }
    let runtime = Runtime::new(&language)?;
    for meaning in runtime.interpret("I will see the river")? {
        assert!(
            runtime
                .analyze(&runtime.realize(&meaning)?)?
                .contains(&meaning)
        );
    }
    Ok(())
}

#[test]
fn reroll_does_not_reshuffle_independent_roots() -> Result<()> {
    let initial = Recipe {
        seed: "stable-roots".into(),
        lexicon_size: 128,
        ..Recipe::default()
    };
    let first = generate(initial.clone())?;
    let mut changed = initial;
    changed.rerolls.insert("lexeme/n.water".into(), 1);
    let second = generate(changed)?;
    for (a, b) in first.lexicon.iter().zip(&second.lexicon) {
        if a.id != "n.water" {
            assert_eq!(a, b);
        }
    }
    assert_eq!(first.grammar, second.grammar);
    Ok(())
}
