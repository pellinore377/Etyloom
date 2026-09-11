use etyloom_core::{Category, English, Error, Result};

pub struct Concept {
    pub id: String,
    pub category: Category,
    pub english: English,
    pub transitive: bool,
    pub frequency: u16,
}

pub fn concepts() -> Result<Vec<Concept>> {
    include_str!("concepts.txt")
        .lines()
        .enumerate()
        .map(|(i, line)| {
            let fields: Vec<_> = line.split('|').collect();
            let kind = fields
                .first()
                .copied()
                .ok_or_else(|| Error::Invalid("Missing concept category".into()))?;
            let base = fields
                .get(1)
                .copied()
                .ok_or_else(|| Error::Invalid("Missing concept gloss".into()))?;
            let category = match kind {
                "n" => Category::Noun,
                "v" => Category::Verb,
                "a" => Category::Adjective,
                _ => return Err(Error::Invalid(format!("Unknown concept category {kind}"))),
            };
            Ok(Concept {
                id: format!("{kind}.{base}"),
                category,
                english: English {
                    base: base.into(),
                    plural: fields.get(2).copied().unwrap_or(base).into(),
                    past: if category == Category::Verb {
                        fields.get(2).copied().unwrap_or(base).into()
                    } else {
                        String::new()
                    },
                    third: fields.get(3).copied().unwrap_or(base).into(),
                },
                transitive: fields.get(4) == Some(&"1"),
                frequency: if i < 40 {
                    8000
                } else if i < 98 {
                    4000
                } else {
                    500
                },
            })
        })
        .collect()
}
