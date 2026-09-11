# Implemented coverage

## What the engine represents

A package stores stable lexical identities, tokenized sound sequences, ancestral and contemporary paradigms, historical event records, a grammar configuration, typed meanings, examples and a content-derived revision. The seed is input, not a database row ID. Author revisions and fictional historical stages are different types of identity.

The current implementation composes a bounded subset of mechanisms, not the entire architecture roadmap. Each capability below has executable behavior; later work must add realization, analysis, provenance and tests together.

| Area | Current implementation | Not yet supported |
|---|---|---|
| Sound | Explicit segments, five vowels plus historical fronted vowels, constrained syllables, three aesthetic biases | Stress tiers, tone, diacritics, user-authored inventories |
| Syntax | Six constituent orders, one adjective per noun phrase, object-case option, transitive/intransitive verbs | Relative clauses, possession, different-subject complements, free ordering |
| Morphology | Particle versus inherited suffix paradigms, contemporary future suffix option, preserved stem alternations | General transducer compiler, agreement, templatic morphology, arbitrary infixation |
| Operators | Present/past/future, negation, same-subject want/can/must, polar questions, coordination | Full aspect/evidentiality/valency systems and arbitrary English constructions |
| History | Ordered whole-paradigm sound laws, frequency-biased renewal, contemporary word creation | Community/contact graph, borrowing UI, semantic drift, grammaticalization, arbitrary dated events |
| Vocabulary | Original editorial concepts plus transparent noun compounds | Empirically curated thousands of senses, semantic selectional restrictions, broad idiom library |
| Translation | Controlled English and conlang analysis, grammatical generation, visible alternatives | Unrestricted English understanding, model inference, confidence calibration |
| Learning | Example-derived exercises, semantic answer matching, revision binding, stored review intervals | Skill-unlocking curriculum, due-item selection UI, audio, pronunciation scoring |
| Authoring | Recipes, component reroll counters, regenerated drafts, publish CAS, immutable revisions, exports/import API | Direct manual word editing, granular impact preview, old-revision browser, family editing |

## Historical algorithm

The engine constructs an ancestor with functional grammar and complete nominal/verb paradigms. Sound laws read the previous segment sequence and write a new one. The second and third developments in an inflecting language can front vowels before an inherited i ending and then remove the ending. The modern paradigm retains the alternation.

At the renewal stage, selected lower-frequency nouns adopt a new productive suffix. Frequent forms preserve their inherited patterns. These are explicit editorial mechanisms, not calibrated predictions of actual historical change.

Contemporary compounds combine current roots and current inflection. They do not receive sound laws that predate their introduction. Their origin is labeled as a compound, including source entry IDs. More lexical entries must not be described as more independently researched meanings.

The current history sequence is constrained and partly shared across generations. A rich solver with arbitrary compatible event composition is subsequent work, not an existing feature.

## Reproducibility

Generation uses domain-separated BLAKE3 randomness addressed by component and entry ID. Integer sampling and stable ordering avoid thread- or hash-map-dependent outcomes. Reroll counters affect only their named random streams, but the linguistic consequences still propagate to dependent forms. A root change can change its compounds; it does not reshuffle every independent root.

Packages contain exact schema, content and engine IDs. Unknown versions fail explicitly. Full packages preserve results without requiring future generator behavior. Automatic schema migration and replay of archived engines are not yet implemented.

## Safety of language claims

A successful parse proves conformance only to the implemented grammar. A round-trip can still share mistakes between generator and parser. Use independent test meanings, negative examples and human review. Do not report a probability that a generated language is natural.

The lesson and translation interfaces must distinguish unknown analysis from a confidently rejected answer. Corpus splits group construction families rather than randomly splitting near-identical sentences. These synthetic corpora still need independently authored test sets before a neural model can claim broad accuracy.
