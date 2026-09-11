use crate::random::Random;
use etyloom_core::{Form, Recipe, Result, Sound, SoundStyle};

pub fn inventory(recipe: &Recipe) -> Result<Vec<Sound>> {
    use Sound::*;
    let mut sounds = vec![P, T, K, M, N, L, S, A, E, I, O, U];
    let extra = match recipe.sound {
        SoundStyle::Fluid => vec![R, Y, W, V, H],
        SoundStyle::Balanced => vec![B, D, G, R, Y, F, H],
        SoundStyle::Crisp => vec![F, Sh, Ng, R, Z, B, D],
    };
    let mut rng = Random::new(recipe, "phonology/inventory");
    for sound in extra {
        if rng.below(4)? != 0 {
            sounds.push(sound);
        }
    }
    Ok(sounds)
}

pub fn word(recipe: &Recipe, sounds: &[Sound], address: &str, short: bool) -> Result<Form> {
    let mut rng = Random::new(recipe, address);
    let consonants: Vec<_> = sounds.iter().copied().filter(|s| !s.vowel()).collect();
    let vowels: Vec<_> = sounds.iter().copied().filter(|s| s.vowel()).collect();
    let syllables = if short {
        1
    } else {
        rng.weighted(&[(2, 7), (3, 3)])?
    };
    let mut form = Vec::new();
    for n in 0..syllables {
        if n > 0 || rng.below(5)? != 0 {
            form.push(rng.pick(&consonants)?);
        }
        form.push(rng.pick(&vowels)?);
        let coda_probability = match recipe.sound {
            SoundStyle::Fluid => 12,
            SoundStyle::Balanced => 5,
            SoundStyle::Crisp => 3,
        };
        if rng.below(coda_probability)? == 0 {
            form.push(rng.pick(&[Sound::N, Sound::S, Sound::L])?);
        }
    }
    Ok(Form(form))
}

#[derive(Debug, Clone, Copy)]
pub enum Law {
    IntervocalicVoicing,
    IFronting,
    FinalILoss,
    FinalDevoicing,
    Spirantization,
}

pub fn apply(form: &Form, law: Law) -> Form {
    use Sound::*;
    let mut output = Vec::with_capacity(form.0.len());
    for (i, sound) in form.0.iter().copied().enumerate() {
        let left = i.checked_sub(1).and_then(|j| form.0.get(j)).copied();
        let right = form.0.get(i + 1).copied();
        let between_vowels = left.is_some_and(Sound::vowel) && right.is_some_and(Sound::vowel);
        let final_sound = i + 1 == form.0.len();
        if matches!(law, Law::FinalILoss) && final_sound && sound == I && form.0.len() > 2 {
            continue;
        }
        let changed = match law {
            Law::IntervocalicVoicing if between_vowels => match sound {
                P => B,
                T => D,
                K => G,
                _ => sound,
            },
            Law::IFronting if form.0.last() == Some(&I) && i + 1 < form.0.len() => match sound {
                A => Ae,
                O => Oe,
                U => Yv,
                _ => sound,
            },
            Law::FinalDevoicing if final_sound => match sound {
                B => P,
                D => T,
                G => K,
                V => F,
                Z => S,
                Zh => Sh,
                _ => sound,
            },
            Law::Spirantization if between_vowels => match sound {
                B => V,
                D => Z,
                _ => sound,
            },
            _ => sound,
        };
        output.push(changed);
    }
    Form(output)
}

pub fn describe(law: Law) -> (&'static str, &'static str, &'static str) {
    match law {
        Law::IntervocalicVoicing => (
            "intervocalic-voicing",
            "Between vowels, stops gain a voice",
            "p, t, k become b, d, g between vowels. Each rule reads its unchanged input, then writes a new form.",
        ),
        Law::IFronting => (
            "i-fronting",
            "A suffix leaves its mark",
            "Before a final i, a, o, u become æ, ø, y. The process affects full inherited paradigms, not only roots.",
        ),
        Law::FinalILoss => (
            "final-i-loss",
            "The old ending disappears",
            "Final i is lost in forms longer than two segments. Earlier vowel alternations remain as evidence of the ending.",
        ),
        Law::FinalDevoicing => (
            "final-devoicing",
            "Word endings lose voicing",
            "Word-final voiced stops and selected fricatives become their voiceless counterparts.",
        ),
        Law::Spirantization => (
            "spirantization",
            "Stops soften between vowels",
            "Intervocalic b and d become v and z in this language's defined sound history.",
        ),
    }
}
