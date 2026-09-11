use crate::{
    app::{Gate, Notice, Session},
    client,
    workspace::JobProgress,
};
use etyloom_core::*;
use leptos::{prelude::*, task::spawn_local};
use leptos_router::{components::A, hooks::use_params_map};
use serde::Deserialize;

#[component]
pub fn LanguagePage() -> impl IntoView {
    view! { <Gate><LanguageBody/></Gate> }
}

#[component]
fn LanguageBody() -> impl IntoView {
    let params = use_params_map();
    let data = RwSignal::new(None::<LanguageDetail>);
    let error = RwSignal::new(None::<String>);
    let reload = RwSignal::new(0u32);
    let generation = RwSignal::new(0u64);
    Effect::new(move |_| {
        let id = params.read().get("id").unwrap_or_default();
        reload.get();
        generation.update(|value| *value += 1);
        let request = generation.get_untracked();
        data.set(None);
        error.set(None);
        spawn_local(async move {
            let result = client::get::<LanguageDetail>(&format!("/api/languages/{id}")).await;
            if generation.is_disposed() || generation.get_untracked() != request {
                return;
            }
            match result {
                Ok(value) => data.set(Some(value)),
                Err(message) => error.set(Some(message)),
            }
        });
    });
    view! {
        <section class="page language-page">
            <A href="/" attr:class="breadcrumb">"← Your languages"</A>
            {move || error.get().map(|message| view! { <Notice message/> })}
            {move || match data.get() {
                None => view! { <p class="loading" role="status">"Opening the language…"</p> }.into_any(),
                Some(language) => {
                    let section = params.read().get("section").unwrap_or_else(|| "overview".into());
                    view! { <LanguageView language section reload/> }.into_any()
                }
            }}
        </section>
    }
}

#[component]
fn LanguageView(language: LanguageDetail, section: String, reload: RwSignal<u32>) -> impl IntoView {
    let id = language.summary.id.clone();
    let revision = language.summary.revision.clone();
    let name = language.summary.name.clone();
    let status = language.summary.status.clone();
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let session = use_context::<Session>();
    let publish_id = id.clone();
    let publish_revision = revision.clone();
    let content = match section.as_str() {
        "overview" => view! { <Overview language=language.clone()/> }.into_any(),
        "lexicon" => view! { <Lexicon language=language.clone()/> }.into_any(),
        "grammar" => view! { <GrammarView language=language.clone()/> }.into_any(),
        "history" => view! { <History language=language.clone()/> }.into_any(),
        "translate" => view! { <Translate language=language.clone()/> }.into_any(),
        "learn" => view! { <Learn language=language.clone()/> }.into_any(),
        "settings" => view! { <Settings language=language.clone()/> }.into_any(),
        _ => view! { <Notice message="This language section does not exist.".into()/> }.into_any(),
    };
    view! {
        <div class="language-heading"><div><p class="eyebrow">"LANGUAGE / "{language.recipe.community.clone()}</p><h1>{name}</h1><p class="language-subtitle">{format!("{} word order · {:?} morphology", language.grammar.order, language.grammar.morphology)}<span class="status">{status.clone()}</span></p></div><div class="language-actions">
            <a class="button secondary small-button" href=format!("/api/languages/{id}/export") download>"Export language ↓"</a>
            {(status == "draft").then(|| view! { <button class="button small-button" disabled=move || busy.get() on:click=move |_| {
                let result = session.ok_or_else(|| "Session unavailable".to_string()).and_then(Session::csrf);
                match result {
                    Err(message) => error.set(Some(message)),
                    Ok(csrf) => { busy.set(true); let id = publish_id.clone(); let revision = publish_revision.clone(); spawn_local(async move {
                        match client::post::<serde_json::Value,_>(&format!("/api/languages/{id}/publish"), &serde_json::json!({"revision":revision}), &csrf).await {
                            Ok(_) => reload.update(|value| *value += 1), Err(message) => error.set(Some(message)),
                        }
                        if !busy.is_disposed() { busy.set(false); }
                    }); }
                }
            }>"Publish revision"</button> })}
        </div></div>
        <div class="context-strip"><span>"SEED "<code>{language.summary.seed.clone()}</code></span><span title=revision.clone()>"REVISION "<code>{short(&revision)}</code></span><span>"HISTORICAL STAGE "<strong>"Contemporary"</strong></span></div>
        <nav class="language-nav" aria-label="Language sections">{[("overview","Overview"),("lexicon","Lexicon"),("grammar","Grammar"),("history","History"),("translate","Translate"),("learn","Learn"),("settings","Recipe")].into_iter().map(|(key,label)| {
            let href = if key == "overview" { format!("/languages/{id}") } else { format!("/languages/{id}/{key}") };
            view! { <A href attr:class=if key == section { "selected" } else { "" }>{label}</A> }
        }).collect_view()}</nav>
        {move || error.get().map(|message| view! { <Notice message/> })}
        {content}
    }
}

#[component]
fn Overview(language: LanguageDetail) -> impl IntoView {
    let id = language.summary.id.clone();
    let example = language.examples.first().cloned();
    let vowel_list = language
        .inventory
        .iter()
        .filter(|s| s.vowel())
        .map(|s| s.ipa())
        .collect::<Vec<_>>()
        .join(" · ");
    let consonants = language
        .inventory
        .iter()
        .filter(|s| !s.vowel())
        .map(|s| s.ipa())
        .collect::<Vec<_>>()
        .join(" · ");
    view! {
        <div class="overview-grid">
            <div class="overview-main">
                <section class="specimen"><div class="section-heading"><p class="eyebrow">"01 / IN ITS OWN WORDS"</p><span class="small muted">"A grammatical specimen"</span></div>
                    {example.map(|e| view! { <blockquote><p class="specimen-text">{e.text}</p><footer>{e.english}</footer></blockquote> })}
                    <A href=format!("/languages/{id}/translate") attr:class="text-link">"Try a sentence of your own →"</A>
                </section>
                <section class="overview-section"><div class="section-heading"><p class="eyebrow">"02 / THE SOUND OF IT"</p><A href=format!("/languages/{id}/grammar") attr:class="text-link">"Grammar reference ↗"</A></div><h2>"A vocabulary of sound."</h2><div class="sound-columns"><div><p class="small muted">"VOWELS / SEGMENTAL IPA"</p><p class="sound-list">{vowel_list}</p></div><div><p class="small muted">"CONSONANTS / SEGMENTAL IPA"</p><p class="sound-list">{consonants}</p></div></div><p class="field-help">"The inventory reflects contemporary dictionary forms. Historical glyphs may differ. Pronunciation audio is not available in this release."</p></section>
                <section class="overview-section"><div class="section-heading"><p class="eyebrow">"03 / WORKING NOTES"</p></div><div class="trait-grid"><div><strong>{language.grammar.order.to_string()}</strong><span>"Constituent order"</span></div><div><strong>{language.summary.entries}</strong><span>"Lexical entries, including compounds"</span></div><div><strong>{language.summary.stages}</strong><span>"Recorded language stages"</span></div></div>
                    <details class="coverage"><summary>"What this language currently supports"</summary><ul>{language.limitations.into_iter().map(|limitation| view! { <li>{limitation}</li> }).collect_view()}</ul></details>
                </section>
            </div>
            <aside class="history-note"><p class="eyebrow">"THE LANGUAGE BEFORE THIS ONE"</p><h2>"A past you"<br/><em>"can follow."</em></h2><div class="mini-timeline">{language.stages.iter().map(|stage| view! { <div class="mini-stage"><span class="timeline-dot"></span><div><span class="small muted">{format!("STAGE {:02}",stage.index)}</span><h3>{stage.name.clone()}</h3><p>{stage.events.first().map(|e| e.title.clone()).unwrap_or_else(|| "The original vocabulary and grammar.".into())}</p></div></div> }).collect_view()}</div><A href=format!("/languages/{id}/history") attr:class="text-link">"Read the complete history →"</A></aside>
        </div>
    }
}

#[derive(Clone, Deserialize)]
struct Words {
    entries: Vec<Entry>,
    total: usize,
    offset: usize,
}

#[component]
fn Lexicon(language: LanguageDetail) -> impl IntoView {
    let id = StoredValue::new(language.summary.id.clone());
    let query = RwSignal::new(String::new());
    let submitted = RwSignal::new(String::new());
    let offset = RwSignal::new(0usize);
    let words = RwSignal::new(None::<Words>);
    let selected = RwSignal::new(None::<Entry>);
    let error = RwSignal::new(None::<String>);
    let serial = RwSignal::new(0u64);
    Effect::new(move |_| {
        let search = submitted.get();
        let start = offset.get();
        serial.update(|value| *value += 1);
        let request = serial.get_untracked();
        let id = id.get_value();
        spawn_local(async move {
            let result = client::get::<Words>(&format!(
                "/api/languages/{id}/words?q={}&offset={start}",
                client::encode_query(&search)
            ))
            .await;
            if serial.is_disposed() || serial.get_untracked() != request {
                return;
            }
            match result {
                Ok(value) => {
                    selected.set(value.entries.first().cloned());
                    words.set(Some(value));
                    error.set(None);
                }
                Err(message) => error.set(Some(message)),
            }
        });
    });
    view! {
        <div class="section-heading"><div><p class="eyebrow">"THE WORKING DICTIONARY"</p><h2>"Words, and what they carry."</h2></div><span class="small muted">{language.summary.entries}" entries"</span></div>
        <form class="search-form" role="search" on:submit=move |event| { event.prevent_default(); offset.set(0); submitted.set(query.get_untracked()); }><label class="sr-only" for="word-search">"Search by meaning or word"</label><input id="word-search" type="search" placeholder="Search a word or meaning…" maxlength="200" on:input=move |event| query.set(event_target_value(&event))/><button class="button secondary small-button" type="submit">"Search"</button></form>
        {move || error.get().map(|message| view! { <Notice message/> })}
        <div class="lexicon-layout"><section class="word-list" aria-label="Dictionary entries"><div class="word-list-heading"><span>"WORD"</span><span>"MEANING"</span></div>
            {move || match words.get() {
                None => view! { <p class="loading" role="status">"Opening the dictionary…"</p> }.into_any(),
                Some(page) => if page.entries.is_empty() { view! { <p class="empty">"No words match this search."</p> }.into_any() } else {
                    page.entries.into_iter().map(|entry| {
                        let key = entry.id.clone();
                        let word = entry.forms.last().map(|p| p.base.text()).unwrap_or_else(|| "Unavailable form".into());
                        let gloss = entry.gloss.clone();
                        let category = category(entry.category);
                        view! { <button class="word-row" class:active=move || selected.get().is_some_and(|s| s.id == key) on:click=move |_| selected.set(Some(entry.clone()))><span class="word-form">{word}<small>{category}</small></span><span>{gloss}</span><span class="word-arrow" aria-hidden="true">"↗"</span></button> }
                    }).collect_view().into_any()
                }
            }}
            <div class="pagination"><button class="text-button" disabled=move || offset.get() == 0 on:click=move |_| offset.update(|value| *value = value.saturating_sub(40))>"← Previous"</button><span class="small muted">{move || words.get().map(|page| format!("{}–{} of {}", if page.total == 0 { 0 } else { page.offset + 1 }, (page.offset+40).min(page.total),page.total)).unwrap_or_default()}</span><button class="text-button" disabled=move || words.get().is_none_or(|page| page.offset+40 >= page.total) on:click=move |_| offset.update(|value| *value += 40)>"Next →"</button></div>
        </section><aside class="word-inspector" aria-live="polite">{move || selected.get().map(|entry| view! { <Inspector entry/> })}</aside></div>
    }
}

#[component]
fn Inspector(entry: Entry) -> impl IntoView {
    let Some(current) = entry.forms.last().cloned() else {
        return view! { <Notice message="This entry has no readable forms.".into()/> }.into_any();
    };
    let origin = match &entry.origin {
        Origin::Root => "Inherited root; generated in the ancestral stage.".into(),
        Origin::Compound { modifier, head } => format!(
            "Contemporary compound of {} + {}. Coined after the recorded sound changes.",
            modifier.trim_start_matches("n."),
            head.trim_start_matches("n.")
        ),
        Origin::Derivation { base, operation } => {
            format!("Derived from {base} through {operation}.")
        }
        Origin::Loan { source, stage } => format!("Borrowed from {source} in stage {stage}."),
    };
    view! {
        <p class="eyebrow">"LEXICAL NOTE"</p><h2 class="inspector-word">{current.base.text()}</h2><p class="ipa">"/"{current.base.ipa()}"/"<span class="part-of-speech">{category(entry.category)}</span></p>
        <div class="definition"><span class="sense-number">"01"</span><p>{entry.gloss.clone()}</p></div>
        <dl class="forms-list"><div><dt>"Base form"</dt><dd>{current.base.text()}</dd></div>{current.plural.map(|form| view! { <div><dt>"Inherited plural"</dt><dd>{form.text()}</dd></div> })}{current.past.map(|form| view! { <div><dt>"Past form"</dt><dd>{form.text()}</dd></div> })}</dl>
        <h3 class="small-heading">"Where it came from"</h3><p>{origin}</p>
        <div class="word-history">{entry.forms.into_iter().map(|form| view! { <div><span class="timeline-dot"></span><span class="small muted">{format!("{:02}",form.stage)}</span><span class="history-form">{form.base.text()}</span><small>{form.plural.map(|p| format!("plural {}",p.text())).unwrap_or_default()}</small></div> }).collect_view()}</div><p class="field-help">"Stage numbers describe the language’s fictional history, not your saved revisions."</p>
    }.into_any()
}

#[component]
fn GrammarView(language: LanguageDetail) -> impl IntoView {
    let grammar = language.grammar;
    let morphology = match grammar.morphology {
        Morphology::Analytic => {
            "Number, past and future are expressed with separate grammatical words. The noun or verb stem remains uninflected for those categories."
        }
        Morphology::Suffixing => {
            "Nouns and verbs carry inherited number and past paradigms. Future marking uses the contemporary suffix. Historical stem alternations and later regularization are stored per entry."
        }
        Morphology::Mixed => {
            "Number and past use inherited inflection, while future and modal constructions use separate grammatical words. Analogy affects a smaller cohort, preserving more inherited paradigms."
        }
    };
    view! {
        <div class="reference-layout"><nav class="reference-toc" aria-label="Grammar contents"><p class="eyebrow">"REFERENCE"</p><a href="#structure">"01 / Sentence structure"</a><a href="#morphology">"02 / Word formation"</a><a href="#operators">"03 / Scope & operators"</a><a href="#examples">"04 / Examples"</a><a href="#coverage">"05 / Current coverage"</a></nav><article class="grammar-reference">
            <section id="structure"><p class="eyebrow">"01 / SENTENCE STRUCTURE"</p><h2>"The shape of a sentence."</h2><p>{format!("The default constituent order is {}. Each clause links a subject to a verb and, when required by that verb, an object.",grammar.order)}</p><div class="grammar-facts"><p>{if grammar.adjective_before { "Adjectives precede the noun they modify." } else { "Adjectives follow the noun they modify." }}</p><p>{if grammar.case_marking { "Objects receive a bound case suffix on their noun or pronoun head." } else { "Grammatical roles are identified by constituent order, without an object-case suffix." }}</p></div></section>
            <section id="morphology"><p class="eyebrow">"02 / WORD FORMATION"</p><h2>"Patterns and their inheritances."</h2><p>{morphology}</p><p>"Current inflected forms are shown in the lexicon. Do not reconstruct them by attaching an ancestral ending: historical changes apply to complete paradigms."</p><table><caption>"Contemporary grammatical markers"</caption><thead><tr><th>"Function"</th><th>"Written form"</th><th>"Segmental IPA"</th></tr></thead><tbody>{grammar.markers.into_iter().filter(|(key,_)| key != "renewed_plural").map(|(key,form)| view! { <tr><td>{key}</td><td class="serif">{form.text()}</td><td class="ipa">"/"{form.ipa()}"/"</td></tr> }).collect_view()}</tbody></table></section>
            <section id="operators"><p class="eyebrow">"03 / SCOPE & OPERATORS"</p><h2>"Meaning is in the order, too."</h2><p>{if grammar.negation_after { "The negative marker follows the expression it negates." } else { "The negative marker precedes the expression it negates." }}</p><p>"Negating a desire is distinct from desiring a negative action. Modal, negative and question constructions wrap a typed meaning rather than substituting individual English words."</p><div class="example-note"><p>"I do not want to walk."</p><p>"I want to not walk."</p><small>"Two different meanings. The translator preserves the distinction."</small></div></section>
            <section id="examples"><p class="eyebrow">"04 / GENERATED EXAMPLES"</p><h2>"In practice."</h2>{language.examples.into_iter().map(|example| view! { <div class="reference-example"><p>{example.text}</p><span>{example.english}</span><small>{example.skill}</small></div> }).collect_view()}</section>
            <section id="coverage"><p class="eyebrow">"05 / COVERAGE"</p><h2>"What this release can say."</h2><ul>{language.limitations.into_iter().map(|line| view! { <li>{line}</li> }).collect_view()}</ul></section>
        </article></div>
    }
}

#[component]
fn History(language: LanguageDetail) -> impl IntoView {
    view! {
        <div class="history-heading"><p class="eyebrow">"THE RECORDED DEVELOPMENT"</p><h2>"Nothing begins"<br/><em>"quite as it ends."</em></h2><p>"These events were executed on the language’s forms and paradigms. They are not a separately generated story."</p></div>
        <div class="history-timeline">{language.stages.into_iter().map(|stage| view! {
            <section class="history-stage"><div class="stage-marker"><span>{format!("{:02}",stage.index)}</span><i></i></div><div class="stage-body"><p class="eyebrow">"HISTORICAL STAGE"</p><h2>{stage.name}</h2>
                {if stage.events.is_empty() { view! { <p>"The initial inventory, vocabulary and executable grammar establish the starting point. This is a functioning ancestor, not a list of roots without syntax."</p> }.into_any() } else { stage.events.into_iter().map(|event| view! { <div class="historical-event"><h3>{event.title}</h3><p>{event.description}</p><p class="event-count">{event.affected}" lexical entries affected"</p></div> }).collect_view().into_any() }}
                <details class="stage-grammar"><summary>"Grammar at this stage"</summary><p>{format!("Order: {} · morphology: {:?} · object case: {}",stage.grammar.order,stage.grammar.morphology,if stage.grammar.case_marking { "marked" } else { "unmarked" })}</p><dl class="forms-list">{stage.grammar.markers.into_iter().map(|(key,form)| view! { <div><dt>{key}</dt><dd>{form.text()}</dd></div> }).collect_view()}</dl></details>
            </div></section>
        }).collect_view()}</div><aside class="notice"><strong>"History is not a revision number."</strong><p>"A historical stage is part of the language’s fictional development. Publishing saves your authored version of that entire history."</p></aside>
    }
}

#[component]
fn Translate(language: LanguageDetail) -> impl IntoView {
    let id = StoredValue::new(language.summary.id);
    let input = RwSignal::new(String::new());
    let reverse = RwSignal::new(false);
    let busy = RwSignal::new(false);
    let output = RwSignal::new(None::<Translation>);
    let error = RwSignal::new(None::<String>);
    let session = use_context::<Session>();
    let name = language.summary.name;
    view! {
        <div class="section-heading"><div><p class="eyebrow">"PUT THE LANGUAGE TO WORK"</p><h2>"A thought, in another form."</h2></div><span class="badge">"GRAMMAR-BACKED"</span></div>
        <p class="reading-intro">"Controlled translation using this saved language. Try a simple sentence, then explore number, tense, questions and the scope of negation."</p>
        <form class="translation-form" on:submit=move |event| {
            event.prevent_default(); error.set(None); output.set(None);
            match session.ok_or_else(|| "Session unavailable".to_string()).and_then(Session::csrf) {
                Err(message) => error.set(Some(message)),
                Ok(csrf) => { busy.set(true); let id = id.get_value(); let body = TranslationRequest { text: input.get_untracked(), reverse: reverse.get_untracked() }; spawn_local(async move {
                    let result = client::post(&format!("/api/languages/{id}/translate"), &body, &csrf).await;
                    if busy.is_disposed() { return; }
                    match result { Ok(value) => output.set(Some(value)), Err(message) => error.set(Some(message)) }
                    busy.set(false);
                }); }
            }
        }><div class="translation-toolbar"><label class="inline-field">"Direction"<select on:change=move |event| { reverse.set(event_target_value(&event) == "reverse"); output.set(None); }><option value="forward">{format!("English → {name}")}</option><option value="reverse">{format!("{name} → English")}</option></select></label><span class="small muted">"Revision "<code>{short(&language.summary.revision)}</code></span></div>
            <div class="translation-panes"><div class="translation-input"><label for="translate-input">"SOURCE"</label><textarea id="translate-input" rows="7" maxlength="4000" required prop:value=move || input.get() placeholder="I see the river" on:input=move |event| input.set(event_target_value(&event))></textarea><button class="button" type="submit" disabled=move || busy.get()>{move || if busy.get() { "Analyzing…" } else { "Translate →" }}</button></div><div class="translation-output" aria-live="polite"><p class="eyebrow">"EXPRESSION"</p>{move || match output.get() {
                Some(value) => view! { <p class="translated-text">{value.output}</p><p class="field-help">{value.explanation}</p>{(!value.alternatives.is_empty()).then(|| view! { <div class="alternatives"><h3>"Other licensed interpretations"</h3>{value.alternatives.into_iter().map(|line| view! { <p>{line}</p> }).collect_view()}</div> })} }.into_any(),
                None => view! { <p class="translation-placeholder">"Your sentence will appear here."</p><p class="field-help">"Ambiguity and unsupported constructions are reported, never silently guessed."</p> }.into_any(),
            }}</div></div>
        </form>
        {move || error.get().map(|message| view! { <Notice message/> })}
        <div class="suggestions"><span class="eyebrow">"TRY A CONTRAST"</span>{["I see the river", "we walked", "I do not want to walk", "I want to not walk"].into_iter().map(|sentence| view! { <button class="suggestion" on:click=move |_| { reverse.set(false); input.set(sentence.into()); }>{sentence}</button> }).collect_view()}</div>
        <aside class="notice"><strong>"Local neural translation is a separate capability."</strong><p>"This release can export aligned examples for local training. It does not claim to understand unrestricted English or supply a trained neural model."</p><a class="text-link" href=format!("/api/languages/{}/corpus", id.get_value()) download>"Export training corpus ↓"</a></aside>
    }
}

#[component]
fn Learn(language: LanguageDetail) -> impl IntoView {
    let id = StoredValue::new(language.summary.id);
    let revision = StoredValue::new(language.summary.revision);
    let index = RwSignal::new(0usize);
    let exercise = RwSignal::new(None::<Exercise>);
    let answer = RwSignal::new(String::new());
    let feedback = RwSignal::new(None::<Feedback>);
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let session = use_context::<Session>();
    Effect::new(move |_| {
        let number = index.get();
        let path = format!(
            "/api/languages/{}/exercise?index={number}&revision={}",
            id.get_value(),
            revision.get_value()
        );
        feedback.set(None);
        answer.set(String::new());
        exercise.set(None);
        spawn_local(async move {
            let result = client::get(&path).await;
            if exercise.is_disposed() {
                return;
            }
            match result {
                Ok(value) => exercise.set(Some(value)),
                Err(message) => error.set(Some(message)),
            }
        });
    });
    view! {
        <div class="lesson-wrap"><div class="section-heading"><p class="eyebrow">"A LITTLE PRACTICE"</p><span class="small muted">{move || format!("EXERCISE {:02}",index.get()+1)}</span></div><h2>"Make the language yours."</h2><p class="intro">"Write the meaning below in "{language.summary.name}"."</p>
            {move || error.get().map(|message| view! { <Notice message/> })}
            <div class="lesson-sheet">{move || exercise.get().map(|item| view! { <p class="eyebrow">{item.skill.to_uppercase()}</p><p class="lesson-prompt">{item.prompt}</p> })}
                <form on:submit=move |event| {
                    event.prevent_default(); error.set(None);
                    let token = session.ok_or_else(|| "Session unavailable".to_string()).and_then(Session::csrf);
                    match (token,exercise.get_untracked()) {
                        (Err(message),_) => error.set(Some(message)),
                        (_,None) => error.set(Some("Wait for the exercise to load".into())),
                        (Ok(csrf),Some(item)) => { busy.set(true); let path = format!("/api/languages/{}/answer",id.get_value()); let body = AnswerRequest { exercise:item.id,revision:item.revision,answer:answer.get_untracked() }; spawn_local(async move {
                            let result = client::post(&path,&body,&csrf).await;
                            if busy.is_disposed() { return; }
                            match result { Ok(value) => feedback.set(Some(value)), Err(message) => error.set(Some(message)) }
                            busy.set(false);
                        }); }
                    }
                }><label for="lesson-answer" class="small-heading">"YOUR EXPRESSION"</label><textarea id="lesson-answer" required rows="3" maxlength="4000" prop:value=move || answer.get() on:input=move |event| answer.set(event_target_value(&event))></textarea><div class="form-footer"><button class="button" disabled=move || busy.get() type="submit">"Check expression"</button><button class="text-button" type="button" on:click=move |_| index.update(|value| *value += 1)>"Next exercise →"</button></div></form>
                {move || feedback.get().map(|value| view! { <div class="lesson-feedback" class:correct=value.correct role="status"><h3>{if value.correct { "That carries the intended meaning." } else { "A different meaning came through." }}</h3><p>{value.explanation}</p><p class="serif">{value.answer}</p></div> })}
            </div><p class="field-help">"Valid alternatives are analyzed, not compared to a single answer string. An unavailable analysis is reported separately from an incorrect answer. Progress is saved for this revision."</p>
        </div>
    }
}

#[component]
fn Settings(language: LanguageDetail) -> impl IntoView {
    let id = StoredValue::new(language.summary.id.clone());
    let revision = StoredValue::new(language.summary.revision.clone());
    let initial = serde_json::to_string_pretty(&language.recipe).map_err(|e| e.to_string());
    let error = RwSignal::new(initial.as_ref().err().cloned());
    let source = RwSignal::new(initial.unwrap_or_default());
    let busy = RwSignal::new(false);
    let job = RwSignal::new(None::<Job>);
    let session = use_context::<Session>();
    view! {
        <div class="reference-narrow"><p class="eyebrow">"PRESERVE THE THREAD"</p><h2>"The recipe behind the language."</h2><p>"The seed, generation settings and exact engine version form a reproducible recipe. Changing it creates a new draft; a published revision is never rewritten."</p><div class="download-links"><a class="button secondary" href=format!("/api/languages/{}/recipe",id.get_value()) download>"Export recipe ↓"</a><a class="button secondary" href=format!("/api/languages/{}/export",id.get_value()) download>"Export complete language ↓"</a></div>
            {move || error.get().map(|message| view! { <Notice message/> })}
            {move || job.get().map(|value| view! { <JobProgress initial=value/> })}
            <form on:submit=move |event| {
                event.prevent_default(); error.set(None);
                let recipe = serde_json::from_str::<Recipe>(&source.get_untracked()).map_err(|e| format!("Invalid recipe: {e}"));
                let token = session.ok_or_else(|| "Session unavailable".to_string()).and_then(Session::csrf);
                match (recipe,token) {
                    (Err(message),_) | (_,Err(message)) => error.set(Some(message)),
                    (Ok(recipe),Ok(csrf)) => { busy.set(true); let path=format!("/api/languages/{}/draft",id.get_value()); let body=serde_json::json!({"revision":revision.get_value(),"recipe":recipe}); spawn_local(async move {
                        let result = client::post(&path,&body,&csrf).await;
                        if busy.is_disposed() { return; }
                        match result { Ok(value) => job.set(Some(value)), Err(message) => error.set(Some(message)) }
                        busy.set(false);
                    }); }
                }
            }><label for="recipe-editor">"GENERATION RECIPE / JSON"</label><textarea id="recipe-editor" class="mono recipe-editor" rows="20" required prop:value=move || source.get() on:input=move |event| source.set(event_target_value(&event))></textarea><p class="field-help">"Advanced editing: a reroll counter such as \"lexeme/n.water\": 1 changes that root and its dependent history. Global grammar choices are addressed independently."</p><button class="button" type="submit" disabled=move || busy.get() || job.get().is_some()>"Generate a new draft →"</button></form>
            <aside class="notice"><strong>"Importing and preserving work"</strong><p>"A recipe recreates the generated state. Keep the complete package as well: it includes every stored paradigm and history. The import API verifies its version, contents and checksum."</p></aside>
        </div>
    }
}

fn short(value: &str) -> String {
    value.chars().take(10).collect()
}
fn category(value: Category) -> &'static str {
    match value {
        Category::Noun => "noun",
        Category::Verb => "verb",
        Category::Adjective => "adjective",
    }
}
