from pathlib import Path


def replace(path, old, new):
    p = Path(path)
    source = p.read_text()
    if old not in source:
        raise ValueError(f'Expected source fragment not found in {path}: {old[:70]}')
    p.write_text(source.replace(old, new, 1))


replace('crates/web/src/lib.rs', '#![forbid(unsafe_code)]', '#![forbid(unsafe_code)]\n#![recursion_limit = "512"]')
for path in ('app', 'workspace', 'language'):
    replace(f'crates/web/src/{path}.rs', 'task::spawn_local', 'task::spawn_local_scoped_with_cancellation as spawn_local')
replace('crates/web/src/workspace.rs', '<option value>', '<option value=value>')
replace('crates/core/src/lib.rs', '    pub past: Option<Form>,\n}', '    pub past: Option<Form>,\n    #[serde(default)]\n    pub future: Option<Form>,\n}')
replace('crates/engine/src/generate.rs', 'past: previous.past.as_ref().map(|f| phonology::apply(f, law)),', 'past: previous.past.as_ref().map(|f| phonology::apply(f, law)),\n                future: previous.future.as_ref().map(|f| phonology::apply(f, law)),')
replace('crates/engine/src/generate.rs', '|| form.past != previous.past', '|| form.past != previous.past\n                || form.future != previous.future')
replace('crates/engine/src/generate.rs', '    Ok(Paradigm {\n        stage,\n        base,\n        plural,\n        past,\n    })', '    let future = if category == Category::Verb && grammar.morphology == Morphology::Suffixing {\n        Some(base.joined(marker(grammar, "future")?))\n    } else { None };\n    Ok(Paradigm { stage, base, plural, past, future })')
replace('crates/engine/src/lib.rs', '.chain(paradigm.past.iter())', '.chain(paradigm.past.iter()).chain(paradigm.future.iter())')
replace('crates/engine/src/lib.rs', 'if sentence != example.text || !runtime.analyze(&sentence)?.contains(&example.meaning) {', 'if sentence != example.text || !runtime.analyze(&sentence)?.contains(&example.meaning) || !runtime.interpret(&example.english)?.contains(&example.meaning) {')
replace('crates/engine/src/grammar.rs', 'let future = form.base.joined(marker(&package.grammar, "future")?).text();', 'let future = form.future.as_ref().ok_or_else(|| Error::Package("Missing future paradigm".into()))?.text();')
replace('crates/engine/src/grammar.rs', 'Ok(vec![form.base.joined(marker(grammar, "future")?).text()])', 'Ok(vec![form.future.as_ref().ok_or_else(|| Error::Package("Missing future paradigm".into()))?.text()])')
replace('crates/engine/src/grammar.rs', 'Modal::Want => "want to", Modal::Can => "can", Modal::Must => "must"', 'Modal::Want => "want to", Modal::Can if bare => "be able to", Modal::Must if bare => "have to", Modal::Can => "can", Modal::Must => "must"')
replace('crates/engine/src/grammar.rs', '            Meaning::Negation { body } => {\n                if let Meaning::Event', '            Meaning::Negation { body } => {\n                if let Meaning::Modal { modal: Modal::Can, body: inner } = body.as_ref() && !bare {\n                    let (subject, predicate) = self.english_predicate(inner, true, depth + 1)?;\n                    return Ok((subject, format!("cannot {predicate}")));\n                }\n                if let Meaning::Event')
replace('crates/engine/src/grammar.rs', '        if ["do", "does", "did", "will"].contains(&first) {', '''        if first == "cannot" {
            return Ok(self.english_parse_predicate(subject, &words[1..], Some(Tense::Present), depth + 1)?.into_iter().map(|body| Meaning::Negation { body: Box::new(Meaning::Modal { modal: Modal::Can, body: Box::new(body) }) }).collect());
        }
        let ability = ["be", "am", "is", "are"].contains(&first) && words.get(1).map(String::as_str) == Some("able") && words.get(2).map(String::as_str) == Some("to");
        let obligation = ["have", "has"].contains(&first) && words.get(1).map(String::as_str) == Some("to");
        if ability || obligation {
            if forced.is_some_and(|tense| tense != Tense::Present) { return Ok(vec![]); }
            let modal = if ability { Modal::Can } else { Modal::Must };
            let offset = if ability { 3 } else { 2 };
            return Ok(self.english_parse_predicate(subject, &words[offset..], Some(Tense::Present), depth + 1)?.into_iter().map(|body| Meaning::Modal { modal, body: Box::new(body) }).collect());
        }
        if ["do", "does", "did", "will"].contains(&first) {''')
replace('crates/web/src/language.rs', 'The noun or verb stem remains uninflected for those categories.', 'The noun or verb stem remains uninflected for those categories.')
replace('crates/web/src/language.rs', 'Future marking uses the contemporary suffix.', 'Future forms follow their inherited, historically developed paradigms.')
