#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SCHEMA: u16 = 1;
pub const ENGINE: &str = "etyloom/0.1.0";
pub const CONTENT: &str = "editorial/0.1.0";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Invalid(String),
    #[error("Unsupported version: {0}")]
    Version(String),
    #[error("Unsupported construction: {0}")]
    Unsupported(String),
    #[error("Unknown word: {0}")]
    Unknown(String),
    #[error("The operation exceeded its fixed search budget")]
    Budget,
    #[error("Generation was canceled; existing revisions are unchanged")]
    Canceled,
    #[error("Invalid language package: {0}")]
    Package(String),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SoundStyle {
    Fluid,
    #[default]
    Balanced,
    Crisp,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Morphology {
    Analytic,
    Suffixing,
    Mixed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Order {
    Svo,
    Sov,
    Vso,
    Vos,
    Ovs,
    Osv,
}

impl std::fmt::Display for Order {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Svo => "SVO",
            Self::Sov => "SOV",
            Self::Vso => "VSO",
            Self::Vos => "VOS",
            Self::Ovs => "OVS",
            Self::Osv => "OSV",
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Recipe {
    pub schema: u16,
    pub engine: String,
    pub content: String,
    pub seed: String,
    pub name: String,
    pub lexicon_size: usize,
    pub history_depth: usize,
    pub sound: SoundStyle,
    pub morphology: Option<Morphology>,
    pub order: Option<Order>,
    pub community: String,
    pub notes: String,
    pub rerolls: BTreeMap<String, u32>,
}

impl Default for Recipe {
    fn default() -> Self {
        Self {
            schema: SCHEMA,
            engine: ENGINE.into(),
            content: CONTENT.into(),
            seed: "first-thread".into(),
            name: "Untitled language".into(),
            lexicon_size: 512,
            history_depth: 4,
            sound: SoundStyle::Balanced,
            morphology: None,
            order: None,
            community: String::new(),
            notes: String::new(),
            rerolls: BTreeMap::new(),
        }
    }
}

impl Recipe {
    pub fn validate(&self) -> Result<()> {
        if self.schema != SCHEMA || self.engine != ENGINE || self.content != CONTENT {
            return Err(Error::Version(format!("{}/{}/{}", self.schema, self.engine, self.content)));
        }
        if self.seed.is_empty() || self.seed.len() > 128 {
            return Err(Error::Invalid("Seed must contain 1–128 bytes".into()));
        }
        if self.name.trim().is_empty() || self.name.chars().count() > 80 {
            return Err(Error::Invalid("Language name must contain 1–80 characters".into()));
        }
        if !(128..=5000).contains(&self.lexicon_size) || !(1..=8).contains(&self.history_depth) {
            return Err(Error::Invalid("Choose 128–5,000 entries and 1–8 historical stages".into()));
        }
        if self.community.len() > 240 || self.notes.len() > 8000 || self.rerolls.len() > 5000 {
            return Err(Error::Invalid("Recipe metadata exceeds its size limit".into()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Sound {
    P, B, T, D, K, G, F, V, S, Z, Sh, Zh, M, N, Ng, L, R, Y, W, H,
    A, E, I, O, U, Ae, Oe, Yv,
}

impl Sound {
    pub fn vowel(self) -> bool {
        matches!(self, Self::A | Self::E | Self::I | Self::O | Self::U | Self::Ae | Self::Oe | Self::Yv)
    }

    pub fn ipa(self) -> &'static str {
        match self {
            Self::P => "p", Self::B => "b", Self::T => "t", Self::D => "d",
            Self::K => "k", Self::G => "ɡ", Self::F => "f", Self::V => "v",
            Self::S => "s", Self::Z => "z", Self::Sh => "ʃ", Self::Zh => "ʒ",
            Self::M => "m", Self::N => "n", Self::Ng => "ŋ", Self::L => "l",
            Self::R => "r", Self::Y => "j", Self::W => "w", Self::H => "h",
            Self::A => "a", Self::E => "e", Self::I => "i", Self::O => "o",
            Self::U => "u", Self::Ae => "æ", Self::Oe => "ø", Self::Yv => "y",
        }
    }

    pub fn spelling(self) -> &'static str {
        match self {
            Self::G => "g", Self::Y => "y", Self::Sh => "š", Self::Zh => "ž",
            Self::Ng => "ŋ", Self::Oe => "ö", Self::Yv => "ü", _ => self.ipa(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Form(pub Vec<Sound>);

impl Form {
    pub fn text(&self) -> String { self.0.iter().map(|s| s.spelling()).collect() }
    pub fn ipa(&self) -> String { self.0.iter().map(|s| s.ipa()).collect() }
    pub fn joined(&self, other: &Self) -> Self {
        Self(self.0.iter().chain(&other.0).copied().collect())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Category { Noun, Verb, Adjective }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct English {
    pub base: String,
    pub plural: String,
    pub past: String,
    pub third: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Origin {
    Root,
    Compound { modifier: String, head: String },
    Derivation { base: String, operation: String },
    Loan { source: String, stage: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Paradigm {
    pub stage: usize,
    pub base: Form,
    pub plural: Option<Form>,
    pub past: Option<Form>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Entry {
    pub id: String,
    pub gloss: String,
    pub category: Category,
    pub transitive: bool,
    pub frequency: u16,
    pub english: English,
    pub origin: Origin,
    pub introduced: usize,
    pub forms: Vec<Paradigm>,
}

impl Entry {
    pub fn current(&self) -> Result<&Paradigm> {
        self.forms.last().ok_or_else(|| Error::Package(format!("{} has no forms", self.id)))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Grammar {
    pub morphology: Morphology,
    pub order: Order,
    pub adjective_before: bool,
    pub case_marking: bool,
    pub negation_after: bool,
    pub markers: BTreeMap<String, Form>,
    pub pronouns: BTreeMap<String, Form>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Event {
    pub id: String,
    pub title: String,
    pub description: String,
    pub affected: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Stage {
    pub index: usize,
    pub name: String,
    pub grammar: Grammar,
    pub events: Vec<Event>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Tense { Present, Past, Future }

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Modal { Want, Can, Must }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Entity {
    Pronoun { person: u8, plural: bool },
    Noun { id: String, plural: bool, adjective: Option<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Meaning {
    Event { subject: Entity, verb: String, object: Option<Entity>, tense: Tense },
    Negation { body: Box<Meaning> },
    Modal { modal: Modal, body: Box<Meaning> },
    Question { body: Box<Meaning> },
    Coordination { left: Box<Meaning>, right: Box<Meaning> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Example {
    pub meaning: Meaning,
    pub english: String,
    pub text: String,
    pub skill: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Package {
    pub schema: u16,
    pub revision: String,
    pub recipe: Recipe,
    pub inventory: Vec<Sound>,
    pub grammar: Grammar,
    pub stages: Vec<Stage>,
    pub lexicon: Vec<Entry>,
    pub examples: Vec<Example>,
    pub capabilities: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Translation {
    pub input: String,
    pub output: String,
    pub alternatives: Vec<String>,
    pub revision: String,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub language_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageSummary {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub seed: String,
    pub revision: String,
    pub status: String,
    pub entries: usize,
    pub stages: usize,
    pub order: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageDetail {
    pub summary: LanguageSummary,
    pub recipe: Recipe,
    pub grammar: Grammar,
    pub inventory: Vec<Sound>,
    pub stages: Vec<Stage>,
    pub examples: Vec<Example>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub authenticated: bool,
    pub display_name: String,
    pub csrf: String,
    pub development: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub state: String,
    pub phase: String,
    pub language_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateRequest { pub project_id: String, pub recipe: Recipe }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationRequest { pub text: String, pub reverse: bool }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exercise {
    pub id: usize,
    pub prompt: String,
    pub revision: String,
    pub skill: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnswerRequest { pub exercise: usize, pub revision: String, pub answer: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feedback { pub correct: bool, pub answer: String, pub explanation: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError { pub error: String, pub request_id: Option<String> }
