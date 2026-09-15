use crate::generate::marker;
use etyloom_core::*;
use std::collections::BTreeMap;

const MAX_ANALYSES: usize = 128;

pub struct Runtime<'a> {
    pub package: &'a Package,
    entries: BTreeMap<&'a str, &'a Entry>,
    nouns: BTreeMap<String, Vec<Entity>>,
    adjectives: BTreeMap<String, Vec<String>>,
    verbs: BTreeMap<String, Vec<(String, Tense)>>,
}

impl<'a> Runtime<'a> {
    pub fn new(package: &'a Package) -> Result<Self> {
        let mut runtime = Self {
            package,
            entries: BTreeMap::new(),
            nouns: BTreeMap::new(),
            adjectives: BTreeMap::new(),
            verbs: BTreeMap::new(),
        };
        for entry in &package.lexicon {
            runtime.entries.insert(&entry.id, entry);
            let form = entry.current()?;
            match entry.category {
                Category::Noun => {
                    runtime
                        .nouns
                        .entry(form.base.text())
                        .or_default()
                        .push(Entity::Noun {
                            id: entry.id.clone(),
                            plural: false,
                            adjective: None,
                        });
                    if let Some(plural) = &form.plural {
                        runtime
                            .nouns
                            .entry(plural.text())
                            .or_default()
                            .push(Entity::Noun {
                                id: entry.id.clone(),
                                plural: true,
                                adjective: None,
                            });
                    }
                }
                Category::Adjective => {
                    runtime
                        .adjectives
                        .entry(form.base.text())
                        .or_default()
                        .push(entry.id.clone());
                }
                Category::Verb => {
                    runtime
                        .verbs
                        .entry(form.base.text())
                        .or_default()
                        .push((entry.id.clone(), Tense::Present));
                    if let Some(past) = &form.past {
                        runtime
                            .verbs
                            .entry(past.text())
                            .or_default()
                            .push((entry.id.clone(), Tense::Past));
                    }
                    if package.grammar.morphology == Morphology::Suffixing {
                        let future = form
                            .future
                            .as_ref()
                            .ok_or_else(|| Error::Package("Missing future paradigm".into()))?
                            .text();
                        runtime
                            .verbs
                            .entry(future)
                            .or_default()
                            .push((entry.id.clone(), Tense::Future));
                    }
                }
            }
        }
        for (key, form) in &package.grammar.pronouns {
            let person = match key.as_bytes().first() {
                Some(b'1') => 1,
                Some(b'2') => 2,
                Some(b'3') => 3,
                _ => return Err(Error::Package("Invalid pronoun key".into())),
            };
            runtime
                .nouns
                .entry(form.text())
                .or_default()
                .push(Entity::Pronoun {
                    person,
                    plural: key.ends_with('p'),
                });
        }
        Ok(runtime)
    }

    fn entry(&self, id: &str) -> Result<&'a Entry> {
        self.entries
            .get(id)
            .copied()
            .ok_or_else(|| Error::Unknown(id.into()))
    }

    fn token(&self, name: &str) -> Result<String> {
        Ok(marker(&self.package.grammar, name)?.text())
    }

    fn modal_first(&self) -> bool {
        !self.package.grammar.negation_after
    }

    fn entity(&self, entity: &Entity, object: bool) -> Result<Vec<String>> {
        let grammar = &self.package.grammar;
        let (mut head, adjective, plural) = match entity {
            Entity::Pronoun { person, plural } => {
                let key = format!("{person}{}", if *plural { "p" } else { "s" });
                (
                    grammar
                        .pronouns
                        .get(&key)
                        .cloned()
                        .ok_or_else(|| Error::Invalid("Unknown pronoun".into()))?,
                    None,
                    false,
                )
            }
            Entity::Noun {
                id,
                plural,
                adjective,
            } => {
                let entry = self.entry(id)?;
                if entry.category != Category::Noun {
                    return Err(Error::Invalid("A noun phrase requires a noun".into()));
                }
                let current = entry.current()?;
                let head = if *plural && grammar.morphology != Morphology::Analytic {
                    current
                        .plural
                        .clone()
                        .ok_or_else(|| Error::Package("Missing plural paradigm".into()))?
                } else {
                    current.base.clone()
                };
                let adjective = adjective
                    .as_ref()
                    .map(|id| {
                        let entry = self.entry(id)?;
                        if entry.category != Category::Adjective {
                            return Err(Error::Invalid("Modifier is not an adjective".into()));
                        }
                        Ok(entry.current()?.base.text())
                    })
                    .transpose()?;
                (
                    head,
                    adjective,
                    *plural && grammar.morphology == Morphology::Analytic,
                )
            }
        };
        if object && grammar.case_marking {
            head = head.joined(marker(grammar, "object")?);
        }
        let mut output = Vec::new();
        if plural {
            output.push(self.token("plural")?);
        }
        if grammar.adjective_before
            && let Some(value) = &adjective
        {
            output.push(value.clone());
        }
        output.push(head.text());
        if !grammar.adjective_before
            && let Some(value) = adjective
        {
            output.push(value);
        }
        Ok(output)
    }

    fn verb(&self, id: &str, tense: Tense) -> Result<Vec<String>> {
        let entry = self.entry(id)?;
        if entry.category != Category::Verb {
            return Err(Error::Invalid("Predicate is not a verb".into()));
        }
        let form = entry.current()?;
        let grammar = &self.package.grammar;
        match tense {
            Tense::Present => Ok(vec![form.base.text()]),
            Tense::Past if grammar.morphology != Morphology::Analytic => Ok(vec![
                form.past
                    .as_ref()
                    .ok_or_else(|| Error::Package("Missing past paradigm".into()))?
                    .text(),
            ]),
            Tense::Future if grammar.morphology == Morphology::Suffixing => Ok(vec![
                form.future
                    .as_ref()
                    .ok_or_else(|| Error::Package("Missing future paradigm".into()))?
                    .text(),
            ]),
            Tense::Past => Ok(vec![self.token("past")?, form.base.text()]),
            Tense::Future => Ok(vec![self.token("future")?, form.base.text()]),
        }
    }

    pub fn realize(&self, meaning: &Meaning) -> Result<String> {
        Ok(self.realize_at(meaning, 0)?.join(" "))
    }

    fn realize_at(&self, meaning: &Meaning, depth: usize) -> Result<Vec<String>> {
        if depth > 8 {
            return Err(Error::Budget);
        }
        match meaning {
            Meaning::Event {
                subject,
                verb,
                object,
                tense,
            } => {
                let entry = self.entry(verb)?;
                if entry.transitive != object.is_some() {
                    return Err(Error::Invalid(format!(
                        "{} requires {} object",
                        entry.gloss,
                        if entry.transitive { "an" } else { "no" }
                    )));
                }
                let subject = self.entity(subject, false)?;
                let predicate = self.verb(verb, *tense)?;
                let object = object
                    .as_ref()
                    .map(|e| self.entity(e, true))
                    .transpose()?
                    .unwrap_or_default();
                let slots = match self.package.grammar.order {
                    Order::Svo => [&subject, &predicate, &object],
                    Order::Sov => [&subject, &object, &predicate],
                    Order::Vso => [&predicate, &subject, &object],
                    Order::Vos => [&predicate, &object, &subject],
                    Order::Ovs => [&object, &predicate, &subject],
                    Order::Osv => [&object, &subject, &predicate],
                };
                Ok(slots.into_iter().flatten().cloned().collect())
            }
            Meaning::Negation { body } => {
                let mut output = self.realize_at(body, depth + 1)?;
                let marker = self.token("not")?;
                if self.package.grammar.negation_after {
                    output.push(marker);
                } else {
                    output.insert(0, marker);
                }
                Ok(output)
            }
            Meaning::Modal { modal, body } => {
                let mut output = self.realize_at(body, depth + 1)?;
                let marker = self.token(modal_key(*modal))?;
                if self.modal_first() {
                    output.insert(0, marker);
                } else {
                    output.push(marker);
                }
                Ok(output)
            }
            Meaning::Question { body } => {
                let mut output = self.realize_at(body, depth + 1)?;
                output.insert(0, self.token("question")?);
                Ok(output)
            }
            Meaning::Coordination { left, right } => {
                let mut output = self.realize_at(left, depth + 1)?;
                output.push(self.token("and")?);
                output.extend(self.realize_at(right, depth + 1)?);
                Ok(output)
            }
        }
    }

    pub fn analyze(&self, input: &str) -> Result<Vec<Meaning>> {
        let tokens = tokenize(input)?;
        let mut budget = 2048;
        let mut meanings = self.analyze_at(&tokens, 0, &mut budget)?;
        meanings.dedup();
        if meanings.is_empty() {
            return Err(Error::Unsupported(
                "No licensed analysis. Check spelling and the supported grammar fragment".into(),
            ));
        }
        Ok(meanings)
    }

    fn analyze_at(
        &self,
        tokens: &[String],
        depth: usize,
        budget: &mut usize,
    ) -> Result<Vec<Meaning>> {
        if depth > 8 || *budget == 0 {
            return Err(Error::Budget);
        }
        *budget -= 1;
        if tokens.is_empty() {
            return Ok(vec![]);
        }
        let mut output = Vec::new();
        for (i, token) in tokens
            .iter()
            .enumerate()
            .filter(|(i, _)| *i > 0 && *i + 1 < tokens.len())
        {
            if token == &self.token("and")? {
                let left = self.analyze_at(&tokens[..i], depth + 1, budget)?;
                let right = self.analyze_at(&tokens[i + 1..], depth + 1, budget)?;
                for a in &left {
                    for b in &right {
                        add(
                            &mut output,
                            Meaning::Coordination {
                                left: Box::new(a.clone()),
                                right: Box::new(b.clone()),
                            },
                        )?;
                    }
                }
            }
        }
        let operations = [
            ("not", !self.package.grammar.negation_after),
            ("want", self.modal_first()),
            ("can", self.modal_first()),
            ("must", self.modal_first()),
            ("question", true),
        ];
        for (key, first) in operations {
            let edge = if first { tokens.first() } else { tokens.last() };
            if edge == Some(&self.token(key)?) && tokens.len() > 1 {
                let inner = if first {
                    &tokens[1..]
                } else {
                    &tokens[..tokens.len() - 1]
                };
                for body in self.analyze_at(inner, depth + 1, budget)? {
                    let body = Box::new(body);
                    let meaning = match key {
                        "not" => Meaning::Negation { body },
                        "question" => Meaning::Question { body },
                        "want" => Meaning::Modal {
                            modal: Modal::Want,
                            body,
                        },
                        "can" => Meaning::Modal {
                            modal: Modal::Can,
                            body,
                        },
                        _ => Meaning::Modal {
                            modal: Modal::Must,
                            body,
                        },
                    };
                    add(&mut output, meaning)?;
                }
            }
        }
        if tokens.len() <= 9 {
            for split in 1..tokens.len() {
                self.parse_event_parts(&[&tokens[..split], &tokens[split..]], &mut output)?;
                for second in split + 1..tokens.len() {
                    self.parse_event_parts(
                        &[&tokens[..split], &tokens[split..second], &tokens[second..]],
                        &mut output,
                    )?;
                }
            }
        }
        Ok(output)
    }

    fn parse_event_parts(&self, parts: &[&[String]], output: &mut Vec<Meaning>) -> Result<()> {
        let transitive = parts.len() == 3;
        let roles: &[usize] = if transitive {
            match self.package.grammar.order {
                Order::Svo => &[0, 1, 2],
                Order::Sov => &[0, 2, 1],
                Order::Vso => &[1, 0, 2],
                Order::Vos => &[2, 0, 1],
                Order::Ovs => &[2, 1, 0],
                Order::Osv => &[1, 2, 0],
            }
        } else {
            match self.package.grammar.order {
                Order::Svo | Order::Sov | Order::Osv => &[0, 1],
                _ => &[1, 0],
            }
        };
        let subject_parts = parts
            .get(*roles.first().ok_or(Error::Budget)?)
            .ok_or(Error::Budget)?;
        let verb_parts = parts
            .get(*roles.get(1).ok_or(Error::Budget)?)
            .ok_or(Error::Budget)?;
        let subjects = self.parse_entity(subject_parts, false)?;
        let verbs = self.parse_verb(verb_parts)?;
        let objects = if transitive {
            self.parse_entity(
                parts
                    .get(*roles.get(2).ok_or(Error::Budget)?)
                    .ok_or(Error::Budget)?,
                true,
            )?
            .into_iter()
            .map(Some)
            .collect()
        } else {
            vec![None]
        };
        for subject in subjects {
            for (verb, tense) in &verbs {
                if self.entry(verb)?.transitive != transitive {
                    continue;
                }
                for object in &objects {
                    add(
                        output,
                        Meaning::Event {
                            subject: subject.clone(),
                            verb: verb.clone(),
                            object: object.clone(),
                            tense: *tense,
                        },
                    )?;
                }
            }
        }
        Ok(())
    }

    fn parse_entity(&self, input: &[String], object: bool) -> Result<Vec<Entity>> {
        if input.is_empty() || input.len() > 3 {
            return Ok(vec![]);
        }
        let mut tokens = input.to_vec();
        let grammar = &self.package.grammar;
        let plural = grammar.morphology == Morphology::Analytic
            && tokens.first() == Some(&self.token("plural")?);
        if plural {
            tokens.remove(0);
        }
        if tokens.is_empty() || tokens.len() > 2 {
            return Ok(vec![]);
        }
        let head_index = if grammar.adjective_before {
            tokens.len() - 1
        } else {
            0
        };
        let Some(head) = tokens.get_mut(head_index) else {
            return Ok(vec![]);
        };
        if object && grammar.case_marking {
            let suffix = self.token("object")?;
            let Some(stem) = head.strip_suffix(&suffix) else {
                return Ok(vec![]);
            };
            *head = stem.to_owned();
        }
        let mut entities = self.nouns.get(head).cloned().unwrap_or_default();
        if plural {
            entities = entities
                .into_iter()
                .filter_map(|e| match e {
                    Entity::Noun { id, adjective, .. } => Some(Entity::Noun {
                        id,
                        plural: true,
                        adjective,
                    }),
                    _ => None,
                })
                .collect();
        }
        if tokens.len() == 2 {
            let index = if head_index == 0 { 1 } else { 0 };
            let Some(adjectives) = tokens.get(index).and_then(|t| self.adjectives.get(t)) else {
                return Ok(vec![]);
            };
            let mut modified = Vec::new();
            for entity in entities {
                if let Entity::Noun { id, plural, .. } = entity {
                    for adjective in adjectives {
                        modified.push(Entity::Noun {
                            id: id.clone(),
                            plural,
                            adjective: Some(adjective.clone()),
                        });
                    }
                }
            }
            entities = modified;
        }
        Ok(entities)
    }

    fn parse_verb(&self, tokens: &[String]) -> Result<Vec<(String, Tense)>> {
        if tokens.len() == 1 {
            return Ok(tokens
                .first()
                .and_then(|t| self.verbs.get(t))
                .cloned()
                .unwrap_or_default());
        }
        if tokens.len() != 2 {
            return Ok(vec![]);
        }
        let tense = if tokens.first() == Some(&self.token("past")?)
            && self.package.grammar.morphology == Morphology::Analytic
        {
            Tense::Past
        } else if tokens.first() == Some(&self.token("future")?)
            && self.package.grammar.morphology != Morphology::Suffixing
        {
            Tense::Future
        } else {
            return Ok(vec![]);
        };
        Ok(tokens
            .get(1)
            .and_then(|t| self.verbs.get(t))
            .into_iter()
            .flatten()
            .filter(|(_, t)| *t == Tense::Present)
            .map(|(id, _)| (id.clone(), tense))
            .collect())
    }

    pub fn english(&self, meaning: &Meaning) -> Result<String> {
        match meaning {
            Meaning::Coordination { left, right } => Ok(format!(
                "{} and {}",
                self.english(left)?,
                self.english(right)?
            )),
            Meaning::Question { body } => Ok(format!("is it true that {}?", self.english(body)?)),
            _ => {
                let (subject, predicate) = self.english_predicate(meaning, false, 0)?;
                Ok(format!("{} {predicate}", self.english_entity(subject)?))
            }
        }
    }

    fn english_entity(&self, entity: &Entity) -> Result<String> {
        match entity {
            Entity::Pronoun { person, plural } => Ok(match (*person, *plural) {
                (1, false) => "I",
                (2, false) => "you",
                (3, false) => "it",
                (1, true) => "we",
                (2, true) => "you all",
                (3, true) => "they",
                _ => return Err(Error::Invalid("Unknown pronoun".into())),
            }
            .into()),
            Entity::Noun {
                id,
                plural,
                adjective,
            } => {
                let entry = self.entry(id)?;
                let head = if *plural {
                    &entry.english.plural
                } else {
                    &entry.english.base
                };
                let modifier = adjective
                    .as_ref()
                    .map(|id| self.entry(id).map(|e| format!("{} ", e.english.base)))
                    .transpose()?
                    .unwrap_or_default();
                Ok(format!("the {modifier}{head}"))
            }
        }
    }

    fn english_predicate<'b>(
        &self,
        meaning: &'b Meaning,
        bare: bool,
        depth: usize,
    ) -> Result<(&'b Entity, String)> {
        if depth > 8 {
            return Err(Error::Budget);
        }
        match meaning {
            Meaning::Event { subject, verb, object, tense } => {
                let entry = self.entry(verb)?;
                if bare && *tense != Tense::Present { return Err(Error::Unsupported("Tensed subordinate predicates are outside the current English interface".into())); }
                let predicate = if bare { entry.english.base.clone() } else { match tense {
                    Tense::Past => entry.english.past.clone(), Tense::Future => format!("will {}", entry.english.base),
                    Tense::Present if third_singular(subject) => entry.english.third.clone(), _ => entry.english.base.clone(),
                }};
                let object = object.as_ref().map(|entity| self.english_entity(entity).map(|text| format!(" {}", object_english(&text)))).transpose()?.unwrap_or_default();
                Ok((subject, format!("{predicate}{object}")))
            }
            Meaning::Modal { modal, body } => {
                let (subject, predicate) = self.english_predicate(body, true, depth + 1)?;
                let verb = match modal { Modal::Want if !bare && third_singular(subject) => "wants to", Modal::Want => "want to", Modal::Can if bare => "be able to", Modal::Must if bare => "have to", Modal::Can => "can", Modal::Must => "must" };
                Ok((subject, format!("{verb} {predicate}")))
            }
            Meaning::Negation { body } => {
                if let Meaning::Modal { modal: Modal::Can, body: inner } = body.as_ref() && !bare {
                    let (subject, predicate) = self.english_predicate(inner, true, depth + 1)?;
                    return Ok((subject, format!("cannot {predicate}")));
                }
                if let Meaning::Event { subject, verb, object, tense } = body.as_ref() {
                    let event = Meaning::Event { subject: subject.clone(), verb: verb.clone(), object: object.clone(), tense: Tense::Present };
                    let (_, predicate) = self.english_predicate(&event, true, depth + 1)?;
                    let auxiliary = if bare { "not" } else { match tense { Tense::Past => "did not", Tense::Future => "will not", Tense::Present if third_singular(subject) => "does not", _ => "do not" }};
                    return Ok((subject, format!("{auxiliary} {predicate}")));
                }
                let (subject, predicate) = self.english_predicate(body, true, depth + 1)?;
                let auxiliary = if bare { "not" } else if third_singular(subject) { "does not" } else { "do not" };
                Ok((subject, format!("{auxiliary} {predicate}")))
            }
            _ => Err(Error::Unsupported("This operator cannot be embedded inside a predicate in the current English interface".into())),
        }
    }

    pub fn interpret(&self, input: &str) -> Result<Vec<Meaning>> {
        let tokens = tokenize(input)?;
        let meanings = self.english_clause(&tokens, 0)?;
        if meanings.is_empty() {
            return Err(Error::Unsupported("Try a sentence such as ‘I see the river’, ‘we walked’, or ‘I want to not walk’. Unknown words and constructions are never invented".into()));
        }
        Ok(meanings)
    }

    fn english_clause(&self, tokens: &[String], depth: usize) -> Result<Vec<Meaning>> {
        if depth > 8 {
            return Err(Error::Budget);
        }
        if tokens.starts_with(&["is".into(), "it".into(), "true".into(), "that".into()]) {
            return Ok(self
                .english_clause(&tokens[4..], depth + 1)?
                .into_iter()
                .map(|body| Meaning::Question {
                    body: Box::new(body),
                })
                .collect());
        }
        if let Some(i) = tokens.iter().position(|s| s == "and") {
            let mut result = Vec::new();
            for left in self.english_clause(&tokens[..i], depth + 1)? {
                for right in self.english_clause(&tokens[i + 1..], depth + 1)? {
                    add(
                        &mut result,
                        Meaning::Coordination {
                            left: Box::new(left.clone()),
                            right: Box::new(right),
                        },
                    )?;
                }
            }
            return Ok(result);
        }
        let mut output = Vec::new();
        let mut words = tokens;
        let question = words
            .first()
            .is_some_and(|s| ["do", "does", "did", "will", "can", "must"].contains(&s.as_str()));
        let auxiliary = if question {
            let first = words.first().cloned();
            words = &words[1..];
            first
        } else {
            None
        };
        for end in 1..=words.len().min(5) {
            for subject in self.english_entities(&words[..end])? {
                let mut predicate = words[end..].to_vec();
                if let Some(auxiliary) = &auxiliary {
                    predicate.insert(0, auxiliary.clone());
                }
                for meaning in self.english_parse_predicate(subject, &predicate, None, depth + 1)? {
                    add(
                        &mut output,
                        if question {
                            Meaning::Question {
                                body: Box::new(meaning),
                            }
                        } else {
                            meaning
                        },
                    )?;
                }
            }
        }
        Ok(output)
    }

    fn english_entities(&self, words: &[String]) -> Result<Vec<Entity>> {
        let phrase = words.join(" ");
        let pronoun = match phrase.as_str() {
            "i" | "me" => Some((1, false)),
            "you" => Some((2, false)),
            "it" => Some((3, false)),
            "we" | "us" => Some((1, true)),
            "you all" => Some((2, true)),
            "they" | "them" => Some((3, true)),
            _ => None,
        };
        if let Some((person, plural)) = pronoun {
            return Ok(vec![Entity::Pronoun { person, plural }]);
        }
        let words = if words
            .first()
            .is_some_and(|w| w == "the" || w == "a" || w == "an")
        {
            &words[1..]
        } else {
            words
        };
        if words.is_empty() {
            return Ok(vec![]);
        }
        let mut output = Vec::new();
        for modifier_count in 0..=1usize.min(words.len().saturating_sub(1)) {
            let adjectives: Vec<Option<String>> = if modifier_count == 0 {
                vec![None]
            } else {
                self.package
                    .lexicon
                    .iter()
                    .filter(|e| {
                        e.category == Category::Adjective && words.first() == Some(&e.english.base)
                    })
                    .map(|e| Some(e.id.clone()))
                    .collect()
            };
            let noun = words[modifier_count..].join(" ");
            for entry in self
                .package
                .lexicon
                .iter()
                .filter(|e| e.category == Category::Noun)
            {
                for plural in [false, true] {
                    if noun.as_str()
                        == if plural {
                            &entry.english.plural
                        } else {
                            &entry.english.base
                        }
                    {
                        for adjective in &adjectives {
                            output.push(Entity::Noun {
                                id: entry.id.clone(),
                                plural,
                                adjective: adjective.clone(),
                            });
                        }
                    }
                }
            }
        }
        Ok(output)
    }

    fn english_parse_predicate(
        &self,
        subject: Entity,
        words: &[String],
        forced: Option<Tense>,
        depth: usize,
    ) -> Result<Vec<Meaning>> {
        if depth > 8 {
            return Err(Error::Budget);
        }
        let Some(first) = words.first().map(String::as_str) else {
            return Ok(vec![]);
        };
        if first == "cannot" {
            return Ok(self
                .english_parse_predicate(subject, &words[1..], Some(Tense::Present), depth + 1)?
                .into_iter()
                .map(|body| Meaning::Negation {
                    body: Box::new(Meaning::Modal {
                        modal: Modal::Can,
                        body: Box::new(body),
                    }),
                })
                .collect());
        }
        let ability = ["be", "am", "is", "are"].contains(&first)
            && words.get(1).map(String::as_str) == Some("able")
            && words.get(2).map(String::as_str) == Some("to");
        let obligation =
            ["have", "has"].contains(&first) && words.get(1).map(String::as_str) == Some("to");
        if ability || obligation {
            if forced.is_some_and(|tense| tense != Tense::Present) {
                return Ok(vec![]);
            }
            let modal = if ability { Modal::Can } else { Modal::Must };
            let offset = if ability { 3 } else { 2 };
            return Ok(self
                .english_parse_predicate(
                    subject,
                    &words[offset..],
                    Some(Tense::Present),
                    depth + 1,
                )?
                .into_iter()
                .map(|body| Meaning::Modal {
                    modal,
                    body: Box::new(body),
                })
                .collect());
        }
        if ["do", "does", "did", "will"].contains(&first) {
            let tense = if first == "did" {
                Tense::Past
            } else if first == "will" {
                Tense::Future
            } else {
                Tense::Present
            };
            if forced.is_some() {
                return Ok(vec![]);
            }
            return self.english_parse_predicate(subject, &words[1..], Some(tense), depth + 1);
        }
        if first == "not" {
            return Ok(self
                .english_parse_predicate(subject, &words[1..], forced, depth + 1)?
                .into_iter()
                .map(|body| Meaning::Negation {
                    body: Box::new(body),
                })
                .collect());
        }
        if ["want", "wants", "can", "must"].contains(&first) {
            if forced.is_some_and(|t| t != Tense::Present) {
                return Ok(vec![]);
            }
            let modal = if first.starts_with("want") {
                Modal::Want
            } else if first == "can" {
                Modal::Can
            } else {
                Modal::Must
            };
            let offset = if modal == Modal::Want {
                if words.get(1).map(String::as_str) != Some("to") {
                    return Ok(vec![]);
                }
                2
            } else {
                1
            };
            return Ok(self
                .english_parse_predicate(
                    subject,
                    &words[offset..],
                    Some(Tense::Present),
                    depth + 1,
                )?
                .into_iter()
                .map(|body| Meaning::Modal {
                    modal,
                    body: Box::new(body),
                })
                .collect());
        }
        let mut result = Vec::new();
        for entry in self
            .package
            .lexicon
            .iter()
            .filter(|e| e.category == Category::Verb)
        {
            let mut tenses = Vec::new();
            if let Some(tense) = forced {
                if first == entry.english.base {
                    tenses.push(tense);
                }
            } else {
                if first == entry.english.base
                    || (third_singular(&subject) && first == entry.english.third)
                {
                    tenses.push(Tense::Present);
                }
                if first == entry.english.past {
                    tenses.push(Tense::Past);
                }
            }
            let objects = if entry.transitive {
                self.english_entities(&words[1..])?
                    .into_iter()
                    .map(Some)
                    .collect()
            } else if words.len() == 1 {
                vec![None]
            } else {
                vec![]
            };
            for tense in tenses {
                for object in &objects {
                    add(
                        &mut result,
                        Meaning::Event {
                            subject: subject.clone(),
                            verb: entry.id.clone(),
                            object: object.clone(),
                            tense,
                        },
                    )?;
                }
            }
        }
        Ok(result)
    }

    pub fn translate(&self, input: &str, reverse: bool) -> Result<Translation> {
        let meanings = if reverse {
            self.analyze(input)?
        } else {
            self.interpret(input)?
        };
        let mut outputs = Vec::new();
        for meaning in meanings {
            let output = if reverse {
                self.english(&meaning)?
            } else {
                self.realize(&meaning)?
            };
            if !outputs.contains(&output) {
                outputs.push(output);
            }
        }
        let output = outputs
            .first()
            .cloned()
            .ok_or_else(|| Error::Unsupported("No translation is available".into()))?;
        let explanation = if outputs.len() > 1 {
            "Multiple licensed interpretations exist. The first result is not a confidence-ranked choice."
        } else {
            "Generated by the saved executable grammar. This is controlled translation, not a neural model."
        };
        Ok(Translation {
            input: input.into(),
            output,
            alternatives: outputs.into_iter().skip(1).collect(),
            revision: self.package.revision.clone(),
            explanation: explanation.into(),
        })
    }
}

fn tokenize(input: &str) -> Result<Vec<String>> {
    if input.len() > 4000 {
        return Err(Error::Invalid(
            "Translation input is limited to 4,000 bytes".into(),
        ));
    }
    let words: Vec<_> = input
        .trim()
        .trim_end_matches(['.', '?', '!'])
        .to_lowercase()
        .split_whitespace()
        .map(String::from)
        .collect();
    if words.is_empty() || words.len() > 64 {
        return Err(Error::Invalid("Enter a sentence of 1–64 words".into()));
    }
    Ok(words)
}

fn modal_key(modal: Modal) -> &'static str {
    match modal {
        Modal::Want => "want",
        Modal::Can => "can",
        Modal::Must => "must",
    }
}
fn third_singular(entity: &Entity) -> bool {
    matches!(
        entity,
        Entity::Pronoun {
            person: 3,
            plural: false
        } | Entity::Noun { plural: false, .. }
    )
}
fn object_english(text: &str) -> &str {
    match text {
        "I" => "me",
        "we" => "us",
        "they" => "them",
        other => other,
    }
}
fn add(output: &mut Vec<Meaning>, meaning: Meaning) -> Result<()> {
    if output.contains(&meaning) {
        return Ok(());
    }
    if output.len() >= MAX_ANALYSES {
        return Err(Error::Budget);
    }
    output.push(meaning);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(index: usize) -> Meaning {
        Meaning::Event {
            subject: Entity::Pronoun {
                person: 1,
                plural: false,
            },
            verb: format!("verb-{index}"),
            object: None,
            tense: Tense::Present,
        }
    }

    #[test]
    fn ambiguity_is_bounded_without_truncating_or_counting_duplicates() -> Result<()> {
        let mut output = Vec::new();
        for index in 0..MAX_ANALYSES {
            add(&mut output, event(index))?;
        }
        add(&mut output, event(0))?;
        assert_eq!(output.len(), MAX_ANALYSES);
        assert!(matches!(
            add(&mut output, event(MAX_ANALYSES)),
            Err(Error::Budget)
        ));
        assert_eq!(output.len(), MAX_ANALYSES);
        Ok(())
    }
}
