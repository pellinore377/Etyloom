# Historical mergers and bounded ambiguity

An additional local sweep covered all 432 combinations of three sound styles, three morphology choices, six constituent orders and eight history depths, using a different seed per case. Every seventeenth case requested 5,000 entries; the others requested 512.

One case initially failed with `Error::Budget`: seed `cross-structure-71`, fluid sounds, suffixing morphology, VSO, eight periods, 512 entries. Its short coordinated sentence for “you will arrive and you walk” has 36 licensed analyses after historical mergers. The former 32-analysis ceiling rejected the entire result during generation validation.

The parser now permits at most 128 distinct analyses. Duplicate analyses do not consume the limit. A further distinct result fails explicitly before growing the result set; there is no silent truncation or claim that the first alternative is the intended meaning. The separate recursion-depth and 2,048-call conlang search limits remain unchanged.

This is a bounded-parser capacity fix, not an algorithmic claim to resolve all natural-language ambiguity. Larger or structurally unsupported inputs can still return a budget or unsupported-construction error.

Verification after the fix:

- All 432 cases in the expanded sweep generated and validated successfully.
- 26 engine tests and six server tests passed locally.
- Native SSR/WASM compile checks and workspace Clippy with warnings denied passed.
- `tests/ambiguity.rs` reproduces the 36-analysis case and checks that the intended meaning and alternative translations remain available.
- The grammar unit test checks duplicate handling, exact capacity and explicit overflow rejection.
- The unchanged Roc v0.1 fixture still preserves its canonical fingerprint. The v0.2 generation algorithm and Roc's new historical counts are unchanged by this parser fix.

These checks supplement the earlier 180-seed sweep and the end-to-end upgrade test; they do not establish complete linguistic coverage. Workflow results remain the source of truth for the final container build and browser verification.
