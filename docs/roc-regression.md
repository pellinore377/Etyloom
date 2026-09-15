# Roc: historical development regression

## Reproduction

Fixture: `crates/engine/tests/fixtures/roc-v1.json`.

The reported recipe requests 5,000 entries, eight developments, crisp sounds, mixed morphology and SVO. Its seed is `30f0a39ba786c97ab11a75e656ec2c5f8247804d82c063c5da44e69fefa50d21`.

The v0.1 engine introduced 4,790 compounds only after the last sound change. Only 210 original entries were present during the history. Later stages replayed already-exhausted rules. Periods 6, 7 and 8 made no inherited-form changes.

The original package fingerprint is `08dcef8343f94d41f251452c9e0d805048b19802304689b0610b58506e39c133`. The updated executable reproduces the original CLI output byte-for-byte when given the unchanged v0.1 recipe.

## Revised generation

New recipes use `etyloom/0.2.0`. A deterministic schedule introduces vocabulary across the historical window, using the stems and grammar that actually exist at its birth. Subsequent changes act on all resident vocabulary, including earlier compounds and their complete paradigms.

A bounded catalog of 14 sound-change mechanisms replaces the fixed repeating sequence. Selection checks actual current forms, resulting grammatical distinctions, and the effect after syllable repair. An exhausted candidate is skipped. When no unused eligible rule remains, the stage is explicitly stable; the engine does not invent changes to inflate the count.

Syllable profiles declare onsets, codas and a repair vowel. They are applied to root generation, compounds, inflection and historical outputs. The current object-case suffix is vowel-initial so it can join a licensed head without introducing a consonant cluster. This is a current parser/generator constraint, not a linguistic universal.

Consonant runs of three segments are not inherently wrong: a permitted coda plus a permitted two-consonant onset can produce one. The regression checks declared constraints, not an arbitrary ban on all long clusters. The old output contained 196 headwords with runs of at least three consonants; that observation alone does not establish that all 196 were unpronounceable.

## Result with the same seed and explicit engine upgrade

| Stage | Inherited entries | Entries changed | Headwords changed | Entries introduced | Grammar forms changed |
|---|---:|---:|---:|---:|---:|
| Ancestral | 0 | 0 | 0 | 1,834 | 0 |
| 1 | 1,834 | 761 | 759 | 392 | 0 |
| 2 | 2,226 | 743 | 131 | 390 | 0 |
| 3 | 2,616 | 491 | 409 | 372 | 0 |
| 4 | 2,988 | 536 | 454 | 394 | 0 |
| 5 | 3,382 | 188 | 188 | 410 | 2 |
| 6 | 3,792 | 155 | 155 | 391 | 0 |
| 7 | 4,183 | 566 | 498 | 416 | 0 |
| 8 | 4,599 | 2,073 | 1,982 | 401 | 1 |

“Entries changed” counts each inherited entry once, including changes only to its inflected forms. Introductions are counted separately. Stage validation independently recomputes the counts from stored forms and rejects inconsistent totals.

## Verification

Local verification passed 24 engine tests and six server tests, native SSR and WASM compile checks, and workspace Clippy with warnings denied. An additional 180-seed release-CLI sweep completed without errors across all three sound styles, all three morphology choices, all six orders and historical depths 1–8. Eighteen sweep recipes requested 5,000 entries; the others requested 512. This is regression coverage, not proof of unlimited linguistic coverage.

The browser regression in `tests/browser/generator-upgrade.spec.cjs` covers imported legacy replay, explicit recipe upgrade, draft generation, distinct change statistics, retained published revisions and a 390 px layout. Its execution result is recorded by the Verify workflow, not inferred from successful compilation.

A local optimized CLI run of the exact upgraded recipe, including validation and pretty-printed JSON output, took **0.55 seconds** and approximately **27 MB peak resident memory**. The environment had a four-CPU quota and 4 GiB memory limit. A separate 20-seed benchmark of 4,096 entries and four developments measured **100 ms p95** inside generation. These are different workloads; neither is a full browser, Docker or production-server latency guarantee, nor a benchmark of the full future roadmap.

```sh
cargo run --release --locked -p etyloom-cli -- generate crates/engine/tests/fixtures/roc-v1.json > original.json
cargo run --release --locked -p etyloom-cli -- upgrade-recipe crates/engine/tests/fixtures/roc-v1.json > upgraded-recipe.json
cargo run --release --locked -p etyloom-cli -- generate upgraded-recipe.json > upgraded.json
cargo run --release --locked -p etyloom-cli -- history upgraded.json
cargo test --locked -p etyloom-engine --test evolution
```

## Using the fix

Update the application image after the publishing workflow succeeds. An unchanged exported recipe intentionally retains the old engine. In the language's **Recipe** page, choose **Upgrade generator**, inspect the recipe, then **Generate a new draft**. An imported recipe has an equivalent explicit upgrade control. Existing published revisions remain stored; they are not silently regenerated.

## Remaining scope

This patch does not implement supplied-word pronunciation editing, audio previews, a larger curated semantic lexicon, changing phonotactic systems over time, community/contact simulation, or neural translation. The 5,000-entry output still includes transparent compounds built from a 210-root editorial concept pack. These limitations remain visible rather than being disguised as completed roadmap features.
