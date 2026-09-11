use etyloom_core::{Package, Recipe};
use etyloom_engine::{Runtime, corpus, generate, verify_import};
use std::{error::Error, fs, io::{self, Write}, time::Instant};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("generate") => {
            let recipe = if let Some(path) = args.get(1) { serde_json::from_slice(&fs::read(path)?)? } else { Recipe::default() };
            let language = generate(recipe)?;
            serde_json::to_writer_pretty(io::stdout().lock(), &language)?;
        }
        Some("recipe") => { serde_json::to_writer_pretty(io::stdout().lock(), &Recipe::default())?; }
        Some("validate") => { let package = load(args.get(1))?; verify_import(&package)?; println!("Valid revision {}", package.revision); }
        Some("translate") | Some("analyze") => {
            let package = load(args.get(1))?;
            let text = args.get(2).ok_or("Provide a sentence as one quoted argument")?;
            let translated = Runtime::new(&package)?.translate(text, args.first().is_some_and(|s| s == "analyze"))?;
            serde_json::to_writer_pretty(io::stdout().lock(), &translated)?;
        }
        Some("corpus") => {
            let package = load(args.get(1))?;
            let count = args.get(2).map(|s| s.parse()).transpose()?.unwrap_or(10_000);
            corpus::write_corpus(&package, count, io::stdout().lock())?;
        }
        Some("bench") => {
            let entries = args.get(1).map(|s| s.parse()).transpose()?.unwrap_or(4096);
            let runs = args.get(2).map(|s| s.parse::<usize>()).transpose()?.unwrap_or(20);
            if runs == 0 || runs > 1000 { return Err("Choose 1–1,000 benchmark runs".into()); }
            let mut elapsed = Vec::new();
            for i in 0..runs {
                let recipe = Recipe { seed: format!("benchmark-{i}"), lexicon_size: entries, ..Recipe::default() };
                let started = Instant::now();
                let language = generate(recipe)?;
                let millis = started.elapsed().as_millis();
                elapsed.push(millis);
                eprintln!("run={i} entries={} elapsed_ms={millis} revision={}", language.lexicon.len(), language.revision);
            }
            elapsed.sort_unstable();
            let index = (runs * 95).div_ceil(100).saturating_sub(1);
            println!("entries={entries} runs={runs} p95_ms={}", elapsed.get(index).ok_or("Missing benchmark result")?);
        }
        _ => { writeln!(io::stderr(), "etyloom recipe | generate [recipe.json] | validate language.json | translate language.json 'English' | analyze language.json 'conlang' | corpus language.json [scenes] | bench [entries] [runs]")?; }
    }
    Ok(())
}

fn load(path: Option<&String>) -> Result<Package, Box<dyn Error>> {
    let path = path.ok_or("Provide a language package path")?;
    let package: Package = serde_json::from_slice(&fs::read(path)?)?;
    verify_import(&package)?;
    Ok(package)
}
