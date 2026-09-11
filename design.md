# Etyloom design and code contract

## Selected direction

Fieldnotes layout. Nocturne brass accent. First-class light and dark themes. This is the production direction; the five-way exploration is retired.

Dark olive masthead, paper-like work surfaces, editorial serif headings and linguistic specimens, compact sans-serif controls, monospaced seeds and revision references. The dictionary, history and language itself take precedence over dashboard chrome. Use vertical historical sequences, reading-width constraints, ruled sections and paired list/inspector layouts.

Light mode uses warm ivory with dark ink. Dark mode uses near-black olive, warm off-white and quiet raised surfaces. Accent fills are muted brass, with separate contrast-safe accent text tokens. Implement theme changes through semantic tokens, preserve the user's override, and respect their system preference initially.

Avoid glassmorphism, ornamental gradients, mascot-led empty states, fake analytics, meaningless waveforms, sparkles, excessive rounded cards and gratuitous animation. No remote fonts or runtime styling CDN. No static mock language in production screens.

## Screen contract

- Worktable: projects, real saved languages, empty state and creation entry point.
- Creation: compact preferences, optional advanced recipe, visible seed behavior, actual phase progress and cancellation.
- Overview: grammatical specimen, contemporary sounds, historical sequence, honest capability summary.
- Lexicon: searchable paginated entries and a selected form's meanings, morphology and recorded history.
- Grammar: reading-first reference generated from actual grammar objects.
- History: executable historical events and their affected forms, separate from author revision history.
- Translate: supported/ambiguous/unknown distinctions and an explicit controlled-grammar boundary.
- Learn: one meaning-focused exercise, licensed-answer analysis and revision-bound progress.
- Recipe: exports and new draft generation, without silently changing a published revision.

## Interaction and accessibility

Semantic controls, keyboard focus, skip navigation, labels, reduced-motion support, status announcements and color-independent state indications. Test 390px and 1440px layouts in both themes, diacritics, long forms, keyboard use and 200% zoom. Body text stays readable. Mobile surfaces reorder by task, not by shrinking a desktop canvas. Inspectors must remain reachable and searches preserve entered values on failure.

All data is rendered as text. Server errors identify the failed operation and next action without exposing secrets. Authentication and ownership are enforced on the server, never just by hiding controls.

## Rust code requirements

Terse means direct and readable, not minified. Follow rustfmt and standard Rust naming and borrowing conventions. Keep functions focused, use precise types and descriptive names, and earn abstractions through real reuse.

Recoverable errors return typed Result values. Use `?` for propagation and explicit handling where recovery differs. Add context at application boundaries, not to every line. Never use runtime unwrap/expect, silent success, unchecked user-driven indexing, or an empty fallback to erase a genuine failure.

Comments are rare and short. Explain non-obvious reasons, safety invariants or external constraints only. No narrative banner comments, tutorial blocks, commented-out code or comments that paraphrase the next line. Public contracts need concise documentation when names/types are insufficient. Architectural explanations live in docs; executable examples live in tests.

CI must test native server and WASM browser feature combinations independently. No `--all-features` shortcut that merges incompatible browser/server worlds. Error exit codes must survive log piping. A green badge is not evidence unless logs actually show completed tests.
