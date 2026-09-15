# Implemented coverage

## What the engine represents

A package stores stable lexical identities, tokenized sound sequences, ancestral and contemporary paradigms, historical event records, a grammar configuration, typed meanings, examples and a content-derived revision. The seed is input, not a database row ID. Author revisions and fictional historical stages are different types of identity.

The current implementation composes a bounded subset of mechanisms, not the entire architecture roadmap. Each capability below has executable behavior; later work must add realization, analysis, provenance and tests together.

| Area | Current implementation | Not yet supported |
|---|---|---|
| Sound | Explicit segments, historical fronted vowels, style-based onset/coda profiles, boundary repair | Stress tiers, tone, arbitrary diacritics, user-authored inventories, supplied-word pronunciation editor |
| Syntax | Six constituent orders, one adjective per noun phrase, object-case option, transitive/intransitive verbs | Relative clauses, possession, different-subject complements, free ordering |
| Morphology | Particle versus inherited suffix paradigms, inherited future forms, preserved stem alternations | General transducer compiler, agreement, templatic morphology, arbitrary infixation |
| Operators | Present/past/future, negation, same-subject want/can/must, polar questions, coordination | Full aspect/evidentiality/valency systems and arbitrary English constructions |
| History | Effect-tested whole-paradigm sound laws, frequency-biased renewal, staged vocabulary births, independently validated change counts | Community/contact graph, borrowing UI, semantic drift, grammaticalization, evolving syllable profiles, arbitrary dated events |
| Vocabulary | 210 original editorial root concepts plus transparent noun compounds | Thousands of curated independent senses, semantic selectional restrictions, broad idiom library |
| Translation | Controlled English and conlang analysis, grammatical generation, visible alternatives | Unrestricted English understanding, model inference, confidence calibration |
| Learning | Example-derived exercises, semantic answer matching, revision binding, stored review intervals | Skill-unlocking curriculum, due-item selection UI, audio, pronunciation scoring |
| Authoring | Recipes, component reroll counters, regenerated drafts, publish CAS, immutable revisions, exports/import API, explicit generator upgrade | Direct manual word editing, granular impact preview, old-revision browser, family editing |

## Historical algorithm

The engine constructs an ancestor with functional grammar and complete nominal/verb paradigms. New v0.2 recipes schedule compounds across the historical window rather than introducing nearly all vocabulary after the last development. A compound uses its source stems and grammar at birth and receives only subsequent changes. Its source entry IDs and introduction stage are retained.

Each period considers unused rules from a bounded 14-mechanism catalog. Eligibility checks actual forms and grammatical control tokens, not merely whether a sound appears somewhere in an inventory. Rules that collapse control distinctions beyond the current parser's support are rejected. Exhausted rules are not repeated as filler. No eligible event produces a recorded stable period.

All forms must satisfy the stage's syllable profile. Explicit vowel insertion repairs prohibited junctions instead of deleting inconvenient segments. The profile remains fixed across the historical window in this release. A sound change can still produce inherited stem alternations, including an i-conditioned vowel change followed later by loss of the ending. Ending loss preserves the only remaining syllable nucleus.

After eligible ending loss, selected lower-frequency nouns can adopt a renewed productive suffix; frequent forms more often retain inherited patterns. These are explicit editorial mechanisms, not calibrated predictions of actual historical change.

Stage statistics separately record resident inherited entries, new births, headword changes, whole-paradigm changes and changed grammar forms. Whole-paradigm counts include inflection-only changes and count each entry once. Import validation recomputes these totals.

More lexical entries must not be described as more independently researched meanings. Rich semantic and contact mechanisms remain subsequent work. See the [Roc regression](roc-regression.md) for the concrete before/after result.

## Reproducibility

Generation uses domain-separated BLAKE3 randomness addressed by component and entry ID. Integer sampling and stable ordering avoid thread- or hash-map-dependent outcomes. Reroll counters affect only their named random streams, but linguistic consequences can propagate to dependent forms and subsequent historical choices.

New recipes select `etyloom/0.2.0`. Explicit v0.1 recipes still execute the archived v0.1 algorithm; the supplied Roc fixture preserves its original serialized output and fingerprint. Unknown versions fail explicitly. Optional v0.2 fields are omitted when serializing legacy packages, preserving their checksums.

Upgrading a recipe is an explicit action that changes its generator version. It does not mutate a saved package. Generating from an upgraded recipe creates a new draft; previously published revisions remain stored. Full package exports remain the durable representation of authored work. General schema migration and arbitrary archived engine versions are not yet implemented.

## Safety of language claims

A successful parse proves conformance only to the implemented grammar. A round-trip can still share mistakes between generator and parser. Use independent test meanings, negative examples and human review. Do not report a probability that a generated language is natural.

The lesson and translation interfaces must distinguish unknown analysis from a confidently rejected answer. Corpus splits group construction families rather than randomly splitting near-identical sentences. These synthetic corpora still need independently authored test sets before a neural model can claim broad accuracy.
