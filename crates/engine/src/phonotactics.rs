use crate::random::Random;
use etyloom_core::*;

pub fn profile(recipe: &Recipe) -> Result<Phonotactics> {
    use Sound::*;
    let consonants = [P, B, T, D, K, G, F, V, S, Z, Sh, Zh, M, N, L, R, Y, W, H];
    let mut onsets = vec![vec![]];
    onsets.extend(consonants.into_iter().map(|sound| vec![sound]));
    if recipe.sound != SoundStyle::Fluid {
        onsets.extend([[P, R], [K, R], [B, R], [G, R], [F, R], [P, L], [K, L]].map(Vec::from));
    }
    if recipe.sound == SoundStyle::Crisp {
        onsets.extend([[T, R], [D, R], [S, T], [S, K], [K, W], [T, W]].map(Vec::from));
    }
    let mut codas = vec![N, M, Ng, L, R];
    if recipe.sound != SoundStyle::Fluid {
        codas.extend([S, Sh, F, V, Z, Zh, T, D, K, G, P, B]);
    }
    let linker = Random::new(recipe, "phonotactics/linker").pick(&[A, E, I])?;
    Ok(Phonotactics {
        onsets,
        codas,
        linker,
    })
}

pub fn validate(profile: &Phonotactics) -> Result<()> {
    if !profile.linker.vowel()
        || profile.onsets.len() > 96
        || profile.codas.len() > 32
        || !profile.onsets.iter().any(Vec::is_empty)
        || profile
            .onsets
            .iter()
            .any(|onset| onset.len() > 2 || onset.iter().any(|s| s.vowel()))
        || profile.codas.iter().any(|s| s.vowel())
    {
        return Err(Error::Package("Invalid syllable constraints".into()));
    }
    Ok(())
}

fn onset(profile: &Phonotactics, sounds: &[Sound]) -> bool {
    profile.onsets.iter().any(|allowed| allowed == sounds)
}

fn coda(profile: &Phonotactics, sounds: &[Sound]) -> bool {
    sounds.is_empty() || sounds.len() == 1 && profile.codas.contains(&sounds[0])
}

fn run_allowed(profile: &Phonotactics, run: &[Sound], initial: bool, final_run: bool) -> bool {
    if initial {
        return onset(profile, run);
    }
    if final_run {
        return coda(profile, run);
    }
    if run.windows(2).any(|pair| pair[0] == pair[1]) {
        return false;
    }
    (0..=run.len().min(1))
        .any(|split| coda(profile, &run[..split]) && onset(profile, &run[split..]))
}

pub fn accepts(profile: &Phonotactics, form: &Form) -> bool {
    if form.0.is_empty() || form.0.len() > 128 || !form.0.iter().any(|s| s.vowel()) {
        return false;
    }
    let mut position = 0;
    while position < form.0.len() {
        if form.0[position].vowel() {
            position += 1;
            continue;
        }
        let end = form.0[position..]
            .iter()
            .position(|s| s.vowel())
            .map_or(form.0.len(), |offset| position + offset);
        if !run_allowed(
            profile,
            &form.0[position..end],
            position == 0,
            end == form.0.len(),
        ) {
            return false;
        }
        position = end;
    }
    true
}

pub fn repair(profile: &Phonotactics, form: &Form) -> Result<Form> {
    if form.0.is_empty() || !form.0.iter().any(|s| s.vowel()) {
        return Err(Error::Package(
            "Cannot syllabify a form without a vowel".into(),
        ));
    }
    let mut output = form.clone();
    let mut position = 0;
    while position < output.0.len() {
        if output.0.len() > 128 {
            return Err(Error::Budget);
        }
        if output.0[position].vowel() {
            position += 1;
            continue;
        }
        let end = output.0[position..]
            .iter()
            .position(|s| s.vowel())
            .map_or(output.0.len(), |offset| position + offset);
        let run = &output.0[position..end];
        if run_allowed(profile, run, position == 0, end == output.0.len()) {
            position = end;
            continue;
        }
        if position == 0 && !onset(profile, &run[..1]) {
            if !coda(profile, &run[..1]) {
                return Err(Error::Package(
                    "A segment cannot be an onset or coda".into(),
                ));
            }
            output.0.insert(0, profile.linker);
            continue;
        }
        let split = (1..=run.len().min(2))
            .rev()
            .find(|&n| run_allowed(profile, &run[..n], position == 0, false))
            .ok_or_else(|| Error::Package("No licensed syllable repair".into()))?;
        output.0.insert(position + split, profile.linker);
        position += split + 1;
    }
    if !accepts(profile, &output) {
        return Err(Error::Package("Syllable repair did not converge".into()));
    }
    Ok(output)
}

pub fn join(profile: &Phonotactics, left: &Form, right: &Form) -> Result<Form> {
    repair(profile, &left.joined(right))
}

pub fn word(
    recipe: &Recipe,
    sounds: &[Sound],
    profile: &Phonotactics,
    address: &str,
    short: bool,
) -> Result<Form> {
    let mut rng = Random::new(recipe, address);
    let onsets: Vec<_> = profile
        .onsets
        .iter()
        .filter(|onset| onset.iter().all(|s| sounds.contains(s)))
        .cloned()
        .collect();
    let vowels: Vec<_> = sounds.iter().copied().filter(|s| s.vowel()).collect();
    let codas: Vec<_> = profile
        .codas
        .iter()
        .copied()
        .filter(|s| sounds.contains(s))
        .collect();
    let syllables = if short {
        1
    } else {
        rng.weighted(&[(2, 8), (3, 2)])?
    };
    let mut output = Vec::new();
    for _ in 0..syllables {
        let cluster = rng.below(if recipe.sound == SoundStyle::Crisp {
            6
        } else {
            10
        })? == 0;
        let choices: Vec<_> = onsets
            .iter()
            .filter(|o| o.len() <= if cluster { 2 } else { 1 })
            .cloned()
            .collect();
        output.extend(rng.pick(&choices)?);
        output.push(rng.pick(&vowels)?);
        let rate = match recipe.sound {
            SoundStyle::Fluid => 10,
            SoundStyle::Balanced => 5,
            SoundStyle::Crisp => 3,
        };
        if !codas.is_empty() && rng.below(rate)? == 0 {
            output.push(rng.pick(&codas)?);
        }
    }
    repair(profile, &Form(output))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repairs_boundaries_without_deleting_segments() -> Result<()> {
        use Sound::*;
        let p = profile(&Recipe::default())?;
        for input in [
            vec![A, L, L, N, A],
            vec![Ng, A],
            vec![A, N, G, R, T, A],
            vec![A, H],
            vec![A, N, S, L],
        ] {
            let original = Form(input);
            let repaired = repair(&p, &original)?;
            assert!(accepts(&p, &repaired));
            assert_eq!(repair(&p, &repaired)?, repaired);
            let mut rest = repaired.0.iter();
            for sound in &original.0 {
                assert!(rest.any(|s| s == sound));
            }
        }
        Ok(())
    }
}
