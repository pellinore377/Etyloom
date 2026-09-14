use crate::{
    phonology::{self, Law},
    phonotactics,
    random::Random,
};
use etyloom_core::*;
use std::collections::BTreeSet;

#[derive(Clone, Copy)]
pub enum Rule {
    Voicing,
    Fronting,
    EndingLoss,
    Devoicing,
    Spirantization,
    Palatalization,
    NasalAssimilation,
    Rhotacism,
    Debuccalization,
    HLoss,
    MidRaising,
    Coalescence,
    CodaLoss,
    FinalVowelLoss,
}

impl Rule {
    pub fn all() -> [Self; 14] {
        use Rule::*;
        [
            Voicing,
            Fronting,
            EndingLoss,
            Devoicing,
            Spirantization,
            Palatalization,
            NasalAssimilation,
            Rhotacism,
            Debuccalization,
            HLoss,
            MidRaising,
            Coalescence,
            CodaLoss,
            FinalVowelLoss,
        ]
    }

    pub fn description(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::Voicing => phonology::describe(Law::IntervocalicVoicing),
            Self::Fronting => phonology::describe(Law::IFronting),
            Self::EndingLoss => phonology::describe(Law::FinalILoss),
            Self::Devoicing => phonology::describe(Law::FinalDevoicing),
            Self::Spirantization => phonology::describe(Law::Spirantization),
            Self::Palatalization => (
                "palatalization",
                "Velars shift before front vowels",
                "k and g become š and ž before e, i, æ, ø or y. Each rule reads its unchanged input.",
            ),
            Self::NasalAssimilation => (
                "nasal-assimilation",
                "Nasals adapt to the next consonant",
                "n becomes m before p, b or m, and ŋ before k or g.",
            ),
            Self::Rhotacism => (
                "intervocalic-rhotacism",
                "An intervocalic lateral becomes a rhotic",
                "l becomes r between vowels.",
            ),
            Self::Debuccalization => (
                "debuccalization",
                "An intervocalic sibilant weakens",
                "s becomes h between vowels.",
            ),
            Self::HLoss => (
                "intervocalic-h-loss",
                "The weakened consonant disappears",
                "Intervocalic h is lost, leaving adjacent vowels.",
            ),
            Self::MidRaising => (
                "mid-vowel-raising",
                "Mid vowels rise",
                "e and o rise to i and u. This is an editorial development, not a prediction.",
            ),
            Self::Coalescence => (
                "vowel-coalescence",
                "Adjacent vowels coalesce",
                "ai becomes e, au becomes o, and identical adjacent vowels merge.",
            ),
            Self::CodaLoss => (
                "liquid-coda-loss",
                "Coda liquids are lost",
                "l and r disappear before a consonant or at a word boundary, provided a vowel remains.",
            ),
            Self::FinalVowelLoss => (
                "final-mid-vowel-loss",
                "Final mid vowels disappear",
                "Final e and o are lost from forms with more than one vowel. Syllable repair retains permitted endings.",
            ),
        }
    }

    pub fn apply(self, form: &Form) -> Form {
        use Sound::*;
        let basic = match self {
            Self::Voicing => Some(Law::IntervocalicVoicing),
            Self::Fronting => Some(Law::IFronting),
            Self::EndingLoss => Some(Law::FinalILoss),
            Self::Devoicing => Some(Law::FinalDevoicing),
            Self::Spirantization => Some(Law::Spirantization),
            _ => None,
        };
        if let Some(law) = basic {
            return phonology::apply(form, law);
        }
        let mut out = Vec::with_capacity(form.0.len());
        let mut position = 0;
        while let Some(&sound) = form.0.get(position) {
            let left = position.checked_sub(1).and_then(|i| form.0.get(i)).copied();
            let right = form.0.get(position + 1).copied();
            let between = left.is_some_and(Sound::vowel) && right.is_some_and(Sound::vowel);
            if matches!(self, Self::Coalescence) {
                let merged = match (sound, right) {
                    (A, Some(I)) => Some(E),
                    (A, Some(U)) => Some(O),
                    (v, Some(next)) if v.vowel() && v == next => Some(v),
                    _ => None,
                };
                if let Some(vowel) = merged {
                    out.push(vowel);
                    position += 2;
                    continue;
                }
            }
            let changed = match self {
                Self::Palatalization if matches!(right, Some(E | I | Ae | Oe | Yv)) => match sound {
                    K => Some(Sh),
                    G => Some(Zh),
                    _ => Some(sound),
                },
                Self::NasalAssimilation if sound == N => match right {
                    Some(P | B | M) => Some(M),
                    Some(K | G) => Some(Ng),
                    _ => Some(sound),
                },
                Self::Rhotacism if sound == L && between => Some(R),
                Self::Debuccalization if sound == S && between => Some(H),
                Self::HLoss if sound == H && between => None,
                Self::MidRaising => Some(match sound {
                    E => I,
                    O => U,
                    _ => sound,
                }),
                Self::CodaLoss
                    if matches!(sound, L | R)
                        && left.is_some_and(Sound::vowel)
                        && !right.is_some_and(Sound::vowel) =>
                {
                    None
                }
                Self::FinalVowelLoss
                    if right.is_none()
                        && matches!(sound, E | O)
                        && form.0[..position].iter().any(|s| s.vowel()) =>
                {
                    None
                }
                _ => Some(sound),
            };
            out.extend(changed);
            position += 1;
        }
        Form(out)
    }
}

pub fn change_form(rule: Rule, profile: &Phonotactics, form: &Form) -> Result<Form> {
    let changed = rule.apply(form);
    phonotactics::repair(profile, &changed)
}

pub fn change_paradigm(
    rule: Rule,
    profile: &Phonotactics,
    form: &Paradigm,
    stage: usize,
) -> Result<Paradigm> {
    Ok(Paradigm {
        stage,
        base: change_form(rule, profile, &form.base)?,
        plural: form
            .plural
            .as_ref()
            .map(|f| change_form(rule, profile, f))
            .transpose()?,
        past: form
            .past
            .as_ref()
            .map(|f| change_form(rule, profile, f))
            .transpose()?,
        future: form
            .future
            .as_ref()
            .map(|f| change_form(rule, profile, f))
            .transpose()?,
    })
}

pub fn changed(a: &Paradigm, b: &Paradigm) -> bool {
    a.base != b.base || a.plural != b.plural || a.past != b.past || a.future != b.future
}

fn change_grammar(rule: Rule, profile: &Phonotactics, grammar: &Grammar) -> Result<Grammar> {
    let mut next = grammar.clone();
    for form in next.markers.values_mut().chain(next.pronouns.values_mut()) {
        *form = change_form(rule, profile, form)?;
    }
    Ok(next)
}

pub fn grammar_usable(grammar: &Grammar) -> bool {
    let mut forms = BTreeSet::new();
    grammar
        .markers
        .iter()
        .filter(|(key, _)| key.as_str() != "renewed_plural")
        .map(|(_, form)| form)
        .chain(grammar.pronouns.values())
        .all(|form| !form.0.is_empty() && forms.insert(form.text()))
        && grammar
            .markers
            .get("object")
            .and_then(|f| f.0.first())
            .is_some_and(|s| s.vowel())
}

pub fn select(
    recipe: &Recipe,
    grammar: &Grammar,
    entries: &[Entry],
    used: &BTreeSet<String>,
    stage: usize,
) -> Result<Option<(Rule, Grammar)>> {
    let profile = grammar
        .phonotactics
        .as_ref()
        .ok_or_else(|| Error::Package("Missing syllable constraints".into()))?;
    let mut candidates = Vec::new();
    for rule in Rule::all() {
        let id = rule.description().0;
        if used.contains(id) || matches!(rule, Rule::EndingLoss) && !used.contains("i-fronting") {
            continue;
        }
        let priority = Random::new(recipe, &format!("history/{stage}/{id}")).below(1_000_000)?;
        candidates.push((priority, id, rule));
    }
    candidates.sort_by_key(|(priority, id, _)| (*priority, *id));
    for (_, _, rule) in candidates {
        let next = change_grammar(rule, profile, grammar)?;
        // Collapsed control tokens exceed this parser's supported ambiguity contract.
        if !grammar_usable(&next) {
            continue;
        }
        let mut effect = next.markers != grammar.markers || next.pronouns != grammar.pronouns;
        if !effect {
            for entry in entries {
                let before = entry.current()?;
                if changed(before, &change_paradigm(rule, profile, before, stage)?) {
                    effect = true;
                    break;
                }
            }
        }
        if effect {
            return Ok(Some((rule, next)));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palatalization_distinguishes_front_vowels_from_the_glide() {
        use Sound::*;
        for vowel in [E, I, Ae, Oe, Yv] {
            assert_eq!(
                Rule::Palatalization.apply(&Form(vec![K, vowel])),
                Form(vec![Sh, vowel])
            );
        }
        let glide = Form(vec![K, Y, A]);
        assert_eq!(Rule::Palatalization.apply(&glide), glide);
    }
}
