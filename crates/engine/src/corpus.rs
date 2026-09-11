use crate::{random::Random, Runtime};
use etyloom_core::*;
use serde::Serialize;
use std::io::Write;

pub fn examples(package: &Package, count: usize) -> Result<Vec<Example>> {
    if count > 100_000 { return Err(Error::Invalid("Corpus limit is 100,000 scenes per export".into())); }
    let runtime = Runtime::new(package)?;
    let verbs: Vec<_> = package.lexicon.iter().filter(|e| e.category == Category::Verb).collect();
    let nouns: Vec<_> = package.lexicon.iter().filter(|e| e.category == Category::Noun && matches!(e.origin, Origin::Root)).collect();
    let adjectives: Vec<_> = package.lexicon.iter().filter(|e| e.category == Category::Adjective).collect();
    let mut output = Vec::with_capacity(count);
    for i in 0..count {
        let mut rng = Random::new(&package.recipe, &format!("corpus/{i}"));
        let verb = rng.pick(&verbs)?;
        let subject = if i % 3 == 0 {
            Entity::Noun { id: rng.pick(&nouns)?.id.clone(), plural: rng.below(2)? == 0, adjective: None }
        } else { Entity::Pronoun { person: (rng.below(3)? + 1) as u8, plural: rng.below(2)? == 0 } };
        let object = if verb.transitive {
            Some(Entity::Noun { id: rng.pick(&nouns)?.id.clone(), plural: rng.below(2)? == 0, adjective: if i % 4 == 0 { Some(rng.pick(&adjectives)?.id.clone()) } else { None } })
        } else { None };
        let tense = if matches!(i % 8, 2 | 3 | 6 | 7) { Tense::Present } else { rng.pick(&[Tense::Present, Tense::Past, Tense::Future])? };
        let event = Meaning::Event { subject: subject.clone(), verb: verb.id.clone(), object, tense };
        let (meaning, skill) = match i % 8 {
            1 => (Meaning::Negation { body: Box::new(event) }, "negation"),
            2 => (Meaning::Modal { modal: Modal::Want, body: Box::new(event) }, "desire"),
            3 => (Meaning::Modal { modal: Modal::Can, body: Box::new(event) }, "ability"),
            4 => (Meaning::Question { body: Box::new(event) }, "questions"),
            5 => (Meaning::Coordination { left: Box::new(event), right: Box::new(Meaning::Event { subject, verb: "v.walk".into(), object: None, tense: Tense::Present }) }, "coordination"),
            6 => (Meaning::Modal { modal: Modal::Want, body: Box::new(Meaning::Negation { body: Box::new(event) }) }, "scope"),
            7 => (Meaning::Negation { body: Box::new(Meaning::Modal { modal: Modal::Want, body: Box::new(event) }) }, "scope"),
            _ => (event, "statements"),
        };
        output.push(Example { english: runtime.english(&meaning)?, text: runtime.realize(&meaning)?, meaning, skill: skill.into() });
    }
    Ok(output)
}

pub fn grade(package: &Package, exercise: usize, revision: &str, answer: &str) -> Result<Feedback> {
    if revision != package.revision { return Err(Error::Invalid("This exercise belongs to a different revision. Reload the lesson".into())); }
    let example = package.examples.get(exercise).ok_or_else(|| Error::Invalid("Unknown exercise".into()))?;
    let runtime = Runtime::new(package)?;
    let analyses = runtime.analyze(answer)?;
    let correct = analyses.contains(&example.meaning);
    Ok(Feedback { correct, answer: example.text.clone(), explanation: if correct { "Your answer has a licensed analysis matching the intended meaning.".into() } else { format!("Your sentence has a different meaning. The intended meaning is: {}", example.english) } })
}

#[derive(Serialize)]
struct Pair<'a> {
    revision: &'a str,
    direction: &'a str,
    split: &'a str,
    source: &'a str,
    target: &'a str,
    meaning: &'a Meaning,
}

pub fn write_corpus(package: &Package, count: usize, mut writer: impl Write) -> Result<()> {
    for example in examples(package, count)? {
        let family = structure(&example.meaning);
        let digest = blake3::hash(family.as_bytes());
        let bucket = usize::from(digest.as_bytes()[0]) % 10;
        let split = match bucket { 0 => "test", 1 => "validation", _ => "train" };
        for reverse in [false, true] {
            let pair = Pair {
                revision: &package.revision, direction: if reverse { "to_english" } else { "from_english" }, split,
                source: if reverse { &example.text } else { &example.english },
                target: if reverse { &example.english } else { &example.text }, meaning: &example.meaning,
            };
            serde_json::to_writer(&mut writer, &pair)?;
            writer.write_all(b"\n").map_err(|e| Error::Invalid(format!("Writing corpus failed: {e}")))?;
        }
    }
    Ok(())
}

fn structure(meaning: &Meaning) -> String {
    match meaning {
        Meaning::Event { subject, object, tense, .. } => format!("event:{tense:?}:{}:{}", entity_shape(subject), object.as_ref().map(entity_shape).unwrap_or_else(|| "none".into())),
        Meaning::Negation { body } => format!("not({})", structure(body)),
        Meaning::Modal { modal, body } => format!("{modal:?}({})", structure(body)),
        Meaning::Question { body } => format!("question({})", structure(body)),
        Meaning::Coordination { left, right } => format!("and({},{})", structure(left), structure(right)),
    }
}

fn entity_shape(entity: &Entity) -> String {
    match entity {
        Entity::Pronoun { person, plural } => format!("pronoun:{person}:{plural}"),
        Entity::Noun { plural, adjective, .. } => format!("noun:{plural}:{}", adjective.is_some()),
    }
}
