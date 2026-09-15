use etyloom_core::*;
use etyloom_engine::{Runtime, generate};

#[test]
fn sound_mergers_can_create_more_than_thirty_two_valid_clause_analyses() -> Result<()> {
    let package = generate(Recipe {
        seed: "cross-structure-71".into(),
        sound: SoundStyle::Fluid,
        morphology: Some(Morphology::Suffixing),
        order: Some(Order::Vso),
        history_depth: 8,
        lexicon_size: 512,
        ..Recipe::default()
    })?;
    let runtime = Runtime::new(&package)?;
    let meanings = runtime.interpret("you will arrive and you walk")?;
    for meaning in meanings {
        let sentence = runtime.realize(&meaning)?;
        let analyses = runtime.analyze(&sentence)?;
        assert_eq!(analyses.len(), 36);
        assert!(analyses.contains(&meaning));
        let translation = runtime.translate(&sentence, true)?;
        assert!(!translation.alternatives.is_empty());
    }
    Ok(())
}
