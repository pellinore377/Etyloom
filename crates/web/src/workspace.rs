use crate::{
    app::{Gate, Notice, Session},
    client,
};
use etyloom_core::*;
use leptos::{prelude::*, task::spawn_local};
use leptos_router::components::A;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[component]
pub fn Workspace() -> impl IntoView {
    view! { <Gate><WorkspaceBody/></Gate> }
}

#[component]
fn WorkspaceBody() -> impl IntoView {
    let projects = RwSignal::new(Vec::<Project>::new());
    let languages = RwSignal::new(Vec::<LanguageSummary>::new());
    let error = RwSignal::new(None::<String>);
    let loading = RwSignal::new(true);
    let project_name = RwSignal::new(String::new());
    let selected = RwSignal::new(String::new());
    let reload = RwSignal::new(0u32);
    let busy = RwSignal::new(false);
    let session = use_context::<Session>();
    Effect::new(move |_| {
        reload.get();
        spawn_local(async move {
            let result = async {
                projects.set(client::get("/api/projects").await?);
                languages.set(client::get("/api/languages").await?);
                Ok::<(), String>(())
            }
            .await;
            if let Err(message) = result {
                error.set(Some(message));
            }
            loading.set(false);
        });
    });
    view! {
        <section class="page">
            <div class="page-heading"><div><p class="eyebrow">"YOUR PRIVATE WORKSHOP"</p><h1>"The worktable."</h1><p class="intro">"A place for languages to take shape."</p></div><A href="/create" attr:class="button">"New language"<span aria-hidden="true">"+"</span></A></div>
            {move || error.get().map(|message| view! { <Notice message/> })}
            <div class="workspace-tools">
                <label class="inline-field">"Project"<select aria-label="Filter by project" on:change=move |event| selected.set(event_target_value(&event))><option value="">"All projects"</option>{move || projects.get().into_iter().map(|project| view! { <option value=project.id>{project.name}</option> }).collect_view()}</select></label>
                <details class="project-create"><summary>"+ New project"</summary><form class="inline-form" on:submit=move |event| {
                    event.prevent_default();
                    let csrf = session.ok_or_else(|| "Session unavailable".to_string()).and_then(Session::csrf);
                    let name = project_name.get_untracked();
                    match csrf {
                        Err(message) => error.set(Some(message)),
                        Ok(csrf) => { busy.set(true); spawn_local(async move {
                            match client::post::<Project,_>("/api/projects", &serde_json::json!({"name": name}), &csrf).await {
                                Ok(_) => { project_name.set(String::new()); reload.update(|n| *n += 1); error.set(None); },
                                Err(message) => error.set(Some(message)),
                            }
                            busy.set(false);
                        }); }
                    }
                }><label>"Project name"<input required maxlength="80" prop:value=move || project_name.get() on:input=move |event| project_name.set(event_target_value(&event))/></label><button class="button small-button" type="submit" disabled=move || busy.get()>"Create project"</button></form></details>
            </div>
            <Show when=move || !loading.get() fallback=|| view! { <p class="loading" role="status">"Loading your languages…"</p> }>
                {move || {
                    let filter = selected.get();
                    let items: Vec<_> = languages.get().into_iter().filter(|language| filter.is_empty() || language.project_id == filter).collect();
                    if items.is_empty() {
                        return view! { <div class="empty ruled"><span class="folio-number">"01 / BEGIN"</span><h2>"A language starts"<br/><em>"with a little curiosity."</em></h2><p>"Choose a sound, leave a seed, and let a history unfold. Your first language will be saved here."</p><A href="/create" attr:class="text-link">"Create your first language →"</A></div> }.into_any();
                    }
                    view! { <div class="language-grid">{items.into_iter().enumerate().map(|(index, language)| {
                        let href = format!("/languages/{}", language.id);
                        view! { <A href attr:class="language-card">
                            <div class="card-meta"><span>{format!("LANGUAGE / {:02}", index + 1)}</span><span class="status">{language.status}</span></div>
                            <h2>{language.name}</h2><p class="card-description">{format!("{} order · {} lexical entries", language.order, language.entries)}</p>
                            <div class="thread-line" aria-hidden="true"><i></i><i></i><i></i><i></i></div>
                            <div class="card-bottom"><span>{format!("{} recorded stages", language.stages)}</span><span aria-hidden="true">"↗"</span></div>
                            <p class="seed-preview">"SEED "<code>{language.seed}</code></p>
                        </A> }
                    }).collect_view()}</div> }.into_any()
                }}
            </Show>
            <aside class="workspace-note"><span class="eyebrow">"THE THREAD IS YOURS"</span><p>"The seed remembers where a language began. A revision preserves what you have made of it."</p></aside>
        </section>
    }
}

#[component]
pub fn CreatePage() -> impl IntoView {
    view! { <Gate><Creation/></Gate> }
}

#[component]
fn Creation() -> impl IntoView {
    let projects = RwSignal::new(Vec::<Project>::new());
    let project = RwSignal::new(String::new());
    let name = RwSignal::new(String::new());
    let seed = RwSignal::new(String::new());
    let sound = RwSignal::new(String::from("balanced"));
    let morphology = RwSignal::new(String::from("auto"));
    let order = RwSignal::new(String::from("auto"));
    let size = RwSignal::new(String::from("512"));
    let depth = RwSignal::new(String::from("4"));
    let community = RwSignal::new(String::new());
    let notes = RwSignal::new(String::new());
    let recipe_json = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);
    let job = RwSignal::new(None::<Job>);
    let session = use_context::<Session>();
    Effect::new(move |_| {
        spawn_local(async move {
            match client::get::<Vec<Project>>("/api/projects").await {
                Ok(values) => {
                    if let Some(first) = values.first() {
                        project.set(first.id.clone());
                    }
                    projects.set(values);
                }
                Err(message) => error.set(Some(message)),
            }
        });
    });
    let submit = move |event: leptos::ev::SubmitEvent| {
        event.prevent_default();
        error.set(None);
        let csrf = session
            .ok_or_else(|| "Session unavailable".to_string())
            .and_then(Session::csrf);
        let result = (|| -> std::result::Result<Recipe, String> {
            if !recipe_json.get_untracked().trim().is_empty() {
                return serde_json::from_str(&recipe_json.get_untracked())
                    .map_err(|e| format!("Invalid generation recipe: {e}"));
            }
            let morphology = match morphology.get_untracked().as_str() {
                "analytic" => Some(Morphology::Analytic),
                "suffixing" => Some(Morphology::Suffixing),
                "mixed" => Some(Morphology::Mixed),
                _ => None,
            };
            let order = match order.get_untracked().as_str() {
                "svo" => Some(Order::Svo),
                "sov" => Some(Order::Sov),
                "vso" => Some(Order::Vso),
                "vos" => Some(Order::Vos),
                "ovs" => Some(Order::Ovs),
                "osv" => Some(Order::Osv),
                _ => None,
            };
            Ok(Recipe {
                name: name.get_untracked(),
                seed: seed.get_untracked(),
                morphology,
                order,
                sound: match sound.get_untracked().as_str() {
                    "fluid" => SoundStyle::Fluid,
                    "crisp" => SoundStyle::Crisp,
                    _ => SoundStyle::Balanced,
                },
                lexicon_size: size
                    .get_untracked()
                    .parse()
                    .map_err(|_| "Vocabulary size must be a number")?,
                history_depth: depth
                    .get_untracked()
                    .parse()
                    .map_err(|_| "History depth must be a number")?,
                community: community.get_untracked(),
                notes: notes.get_untracked(),
                ..Recipe::default()
            })
        })();
        match (csrf, result) {
            (Err(message), _) | (_, Err(message)) => error.set(Some(message)),
            (Ok(csrf), Ok(recipe)) => {
                busy.set(true);
                let mut project_id = project.get_untracked();
                spawn_local(async move {
                    let result = async {
                        if project_id.is_empty() {
                            let new: Project = client::post(
                                "/api/projects",
                                &serde_json::json!({"name": "My languages"}),
                                &csrf,
                            )
                            .await?;
                            project_id = new.id;
                        }
                        client::post::<Job, _>(
                            "/api/generate",
                            &GenerateRequest { project_id, recipe },
                            &csrf,
                        )
                        .await
                    }
                    .await;
                    match result {
                        Ok(value) => job.set(Some(value)),
                        Err(message) => error.set(Some(message)),
                    }
                    busy.set(false);
                });
            }
        }
    };
    view! {
        <section class="page">
            <A href="/" attr:class="breadcrumb">"← Your languages"</A>
            <div class="page-heading"><div><p class="eyebrow">"A NEW THREAD"</p><h1>"Start with a feeling."</h1><p class="intro">"A few choices now. A language with a history next."</p></div><span class="margin-number">"01—04"</span></div>
            {move || error.get().map(|message| view! { <Notice message/> })}
            {move || job.get().map(|job| view! { <JobProgress initial=job/> })}
            <div class="creation-layout">
                <form class="creation-form" on:submit=submit>
                    <fieldset disabled=move || busy.get() || job.get().is_some()>
                        <section class="form-section"><div class="section-index">"01"</div><div><h2>"Give it a name."</h2><div class="form-grid"><label>"Language name"<input name="name" placeholder="A name you can change later" maxlength="80" required=move || recipe_json.get().trim().is_empty() on:input=move |event| name.set(event_target_value(&event))/></label><label>"Project"<select on:change=move |event| project.set(event_target_value(&event))>{move || if projects.get().is_empty() { view! { <option value="">"My languages (created automatically)"</option> }.into_any() } else { projects.get().into_iter().map(|p| view! { <option value=p.id>{p.name}</option> }).collect_view().into_any() }}</select></label></div></div></section>
                        <section class="form-section"><div class="section-index">"02"</div><div><h2>"How should it feel?"</h2><div class="form-grid"><label>"Sound character"<select name="sound" on:change=move |event| sound.set(event_target_value(&event))><option value="balanced">"Balanced — varied and grounded"</option><option value="fluid">"Fluid — open and flowing"</option><option value="crisp">"Crisp — textured and distinct"</option></select></label><label>"Word formation"<select on:change=move |event| morphology.set(event_target_value(&event))><option value="auto">"Let the engine choose"</option><option value="analytic">"Mostly separate words"</option><option value="suffixing">"Expressive endings"</option><option value="mixed">"Mixed, with inherited patterns"</option></select></label></div></div></section>
                        <section class="form-section"><div class="section-index">"03"</div><div><h2>"Leave room for a past."</h2><div class="form-grid"><label>"Lexical entries"<select name="size" on:change=move |event| size.set(event_target_value(&event))><option value="512">"512 — a small working language"</option><option value="128">"128 — a compact exploration"</option><option value="2048">"2,048 — an expanded vocabulary"</option><option value="4096">"4,096 — a broad collection"</option><option value="5000">"5,000 — maximum vocabulary"</option></select></label><label>"Historical developments"<select on:change=move |event| depth.set(event_target_value(&event))><option value="4">"4 stages — inherited alternations and renewal"</option><option value="1">"1 stage — a recent beginning"</option><option value="3">"3 stages — sound changes leave a trace"</option><option value="6">"6 stages — a longer recorded history"</option><option value="8">"8 stages — the full current sequence"</option></select></label></div><p class="field-help">"Larger vocabularies include transparent compounds. Complexity comes from interacting rules, not an entry count."</p></div></section>
                        <section class="form-section"><div class="section-index">"04"</div><div><h2>"Keep the thread."</h2><label>"Seed (optional)"<input name="seed" class="mono" placeholder="Leave blank for a new seed" maxlength="128" on:input=move |event| seed.set(event_target_value(&event))/></label><p class="field-help">"The same seed, settings and engine version reproduce the same language. Export the recipe to preserve all three."</p></div></section>
                        <details class="advanced"><summary>"Advanced choices & imported recipe"</summary><div class="advanced-content"><label>"Constituent order"<select on:change=move |event| order.set(event_target_value(&event))><option value="auto">"Choose automatically"</option>{["svo","sov","vso","vos","ovs","osv"].into_iter().map(|value| view! { <option value>{value.to_uppercase()}</option> }).collect_view()}</select></label><label>"Speech community"<input maxlength="240" on:input=move |event| community.set(event_target_value(&event))/></label><label>"World notes (reference only)"<textarea rows="4" maxlength="8000" on:input=move |event| notes.set(event_target_value(&event))></textarea></label><p class="field-help">"Notes are preserved, not automatically interpreted or used to invent cultural rules."</p><label>"Paste an exported recipe"<textarea class="mono" rows="6" placeholder="Complete recipe JSON overrides the choices above" on:input=move |event| recipe_json.set(event_target_value(&event))></textarea></label></div></details>
                        <div class="form-footer"><button class="button" type="submit">{move || if busy.get() { "Submitting…" } else { "Generate language" }}<span aria-hidden="true">"↗"</span></button><span class="small muted">"Your result is saved as a draft."</span></div>
                    </fieldset>
                </form>
                <aside class="creation-aside"><div class="notebook-note"><p class="eyebrow">"WHAT TAKES SHAPE"</p><h3>"More than"<br/><em>"a list of words."</em></h3><ol class="plain-steps"><li>"A consistent sound system"</li><li>"Grammar you can use"</li><li>"Historical forms you can inspect"</li><li>"A reproducible, exportable revision"</li></ol><div class="note-rule"></div><p>"The engine builds an ancestor, develops complete word paradigms, and preserves the history behind the contemporary forms."</p><p class="small muted">"Early-release coverage is shown with every result. Unsupported grammar is never disguised as a successful translation."</p></div></aside>
            </div>
        </section>
    }
}

#[component]
pub fn JobProgress(initial: Job) -> impl IntoView {
    let job = RwSignal::new(initial);
    let error = RwSignal::new(None::<String>);
    let session = use_context::<Session>();
    let stopped = Arc::new(AtomicBool::new(false));
    let cleanup = stopped.clone();
    on_cleanup(move || cleanup.store(true, Ordering::Relaxed));
    Effect::new(move |_| {
        let stopped = stopped.clone();
        let id = job.get_untracked().id;
        spawn_local(async move {
            while !stopped.load(Ordering::Relaxed) {
                match client::get::<Job>(&format!("/api/jobs/{id}")).await {
                    Ok(value) => {
                        let done =
                            matches!(value.state.as_str(), "completed" | "failed" | "canceled");
                        if stopped.load(Ordering::Relaxed) {
                            break;
                        }
                        job.set(value);
                        if done {
                            break;
                        }
                    }
                    Err(message) => {
                        if !stopped.load(Ordering::Relaxed) {
                            error.set(Some(message));
                        }
                        break;
                    }
                }
                client::pause(1000).await;
            }
        });
    });
    view! {
        <section class="job-panel" aria-live="polite">
            <div class="job-heading"><span class="eyebrow">{move || format!("GENERATION / {}", job.get().state.to_uppercase())}</span><Show when=move || matches!(job.get().state.as_str(), "queued" | "running")><button class="text-button" on:click=move |_| {
                let result = session.ok_or_else(|| "Session unavailable".to_string()).and_then(Session::csrf);
                match result {
                    Err(message) => error.set(Some(message)),
                    Ok(csrf) => { let id = job.get_untracked().id; spawn_local(async move { if let Err(message) = client::post::<serde_json::Value,_>(&format!("/api/jobs/{id}/cancel"), &serde_json::json!({}), &csrf).await { error.set(Some(message)); } }); }
                }
            }>"Cancel"</button></Show></div>
            <p class="job-phase">{move || job.get().phase}</p>
            {move || job.get().error.map(|message| view! { <Notice message/> })}
            {move || error.get().map(|message| view! { <Notice message/> })}
            {move || if job.get().state == "completed" { job.get().language_id.map(|id| view! { <A href=format!("/languages/{id}") attr:class="button">"Open language →"</A> }) } else { None }}
        </section>
    }
}

#[component]
pub fn JobsPage() -> impl IntoView {
    view! { <Gate><Activity/></Gate> }
}

#[component]
fn Activity() -> impl IntoView {
    let jobs = RwSignal::new(Vec::<Job>::new());
    let error = RwSignal::new(None::<String>);
    Effect::new(move |_| {
        spawn_local(async move {
            match client::get("/api/jobs").await {
                Ok(value) => jobs.set(value),
                Err(message) => error.set(Some(message)),
            }
        });
    });
    view! { <section class="page"><p class="eyebrow">"WORKSHOP RECORD"</p><h1>"Activity."</h1><p class="intro">"Generation keeps its place, even when you leave the page."</p>{move || error.get().map(|message| view! { <Notice message/> })}{move || if jobs.get().is_empty() { view! { <p class="empty ruled">"No generation jobs yet."</p> }.into_any() } else { jobs.get().into_iter().map(|job| view! { <JobProgress initial=job/> }).collect_view().into_any() }}</section> }
}
