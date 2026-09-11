use crate::{client, language::LanguagePage, workspace::{CreatePage, JobsPage, Workspace}};
use etyloom_core::SessionInfo;
use leptos::{prelude::*, task::spawn_local};
use leptos_meta::{MetaTags, provide_meta_context};
use leptos_router::{components::{A, Route, Router, Routes}, path};

#[derive(Clone, Copy)]
pub struct Session(pub RwSignal<Option<SessionInfo>>);

impl Session {
    pub fn csrf(self) -> Result<String, String> {
        self.0.get_untracked().filter(|s| s.authenticated).map(|s| s.csrf).ok_or_else(|| "Sign in to continue".into())
    }
}

pub fn shell(_options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <meta name="color-scheme" content="light dark"/>
                <meta name="description" content="A workshop for languages with a past. Create, study and preserve your own languages."/>
                <title>"Etyloom — a language workshop"</title>
                <link rel="icon" href="/icon.svg" type="image/svg+xml"/>
                <link rel="stylesheet" href="/app.css"/>
                <script src="/theme.js"></script>
                <MetaTags/>
            </head>
            <body><App/><script type="module" src="/boot.js"></script></body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    let session = Session(RwSignal::new(None));
    let error = RwSignal::new(None::<String>);
    provide_context(session);
    Effect::new(move |_| {
        spawn_local(async move {
            match client::get::<SessionInfo>("/api/session").await {
                Ok(value) => session.0.set(Some(value)),
                Err(message) => error.set(Some(message)),
            }
        });
    });
    view! {
        <Router>
            <a class="skip-link" href="#main">"Skip to content"</a>
            <header class="masthead">
                <div class="masthead-inner">
                    <A href="/" attr:class="wordmark"><span class="mark" aria-hidden="true">"ē"</span>"etyloom"<span class="edition">"WORKSHOP / 01"</span></A>
                    <nav aria-label="Workspace" class="workspace-nav">
                        <A href="/" exact=true>"Languages"</A>
                        <A href="/jobs">"Activity"</A>
                        <ThemeToggle/>
                        <button class="quiet account" on:click=move |_| {
                            match session.csrf() {
                                Ok(csrf) => spawn_local(async move {
                                    match client::post::<serde_json::Value, _>("/auth/logout", &serde_json::json!({}), &csrf).await {
                                        Ok(_) => session.0.set(Some(SessionInfo { authenticated: false, display_name: String::new(), csrf: String::new(), development: false })),
                                        Err(message) => error.set(Some(message)),
                                    }
                                }),
                                Err(_) => if let Err(message) = client::navigate("/auth/login") { error.set(Some(message)); },
                            }
                        }>{move || if session.0.get().is_some_and(|s| s.authenticated) { "Sign out" } else { "Sign in" }}</button>
                    </nav>
                </div>
            </header>
            <main id="main" tabindex="-1">
                {move || error.get().map(|message| view! { <div class="page"><Notice message/></div> })}
                <Routes fallback=|| view! { <div class="page empty"><h1>"This page is uncharted."</h1><A href="/" attr:class="button">"Return to the worktable"</A></div> }>
                    <Route path=path!("") view=Workspace/>
                    <Route path=path!("create") view=CreatePage/>
                    <Route path=path!("jobs") view=JobsPage/>
                    <Route path=path!("languages/:id") view=LanguagePage/>
                    <Route path=path!("languages/:id/:section") view=LanguagePage/>
                </Routes>
            </main>
            <footer class="site-footer"><span>"ETYLOOM"</span><span>"Every word has a history."</span><span>"Deterministic engine · early release"</span></footer>
        </Router>
    }
}

#[component]
pub fn Gate(children: ChildrenFn) -> impl IntoView {
    let Some(session) = use_context::<Session>() else { return view! { <Notice message="The session context is missing. Reload the page.".into()/> }.into_any(); };
    view! {
        <Show when=move || session.0.get().is_some_and(|s| s.authenticated) fallback=move || {
            if session.0.get().is_none() { return view! { <div class="page loading" role="status">"Opening your workshop…"</div> }.into_any(); }
            view! {
                <section class="page sign-in">
                    <p class="eyebrow">"A LANGUAGE WORKSHOP"</p>
                    <h1>"Give your world"<br/><em>"a voice."</em></h1>
                    <p class="intro">"Create languages with their own grammar, words and history. Keep the forms you love. Discover how they came to be."</p>
                    <a class="button" href="/auth/login">"Open your workshop"<span aria-hidden="true">"↗"</span></a>
                    <p class="small muted">{move || if session.0.get().is_some_and(|s| s.development) { "Local development sign-in. Not available in release builds." } else { "Secure sign-in through your Pocket ID." }}</p>
                    <div class="signin-note"><span class="eyebrow">"A NOTE ON THE ENGINE"</span><p>"Not random words on a page. A saved, executable language: reproducible by seed and inspectable through its history."</p></div>
                </section>
            }.into_any()
        }>{children()}</Show>
    }.into_any()
}

#[component]
pub fn Notice(message: String) -> impl IntoView {
    view! { <div class="notice error" role="alert"><strong>"Something needs attention."</strong><p>{message}</p></div> }
}

#[component]
fn ThemeToggle() -> impl IntoView {
    let current = RwSignal::new(String::from("light"));
    let error = RwSignal::new(None::<String>);
    Effect::new(move |_| match client::theme(false) { Ok(value) => current.set(value), Err(message) => error.set(Some(message)) });
    view! {
        <button class="theme-toggle quiet" aria-label="Toggle light and dark mode" title="Switch theme" on:click=move |_| {
            match client::theme(true) { Ok(value) => { current.set(value); error.set(None); }, Err(message) => error.set(Some(message)) }
        }><span aria-hidden="true">{move || if current.get() == "dark" { "☼" } else { "◐" }}</span><span class="theme-label">{move || if current.get() == "dark" { "Light" } else { "Dark" }}</span></button>
        {move || error.get().map(|message| view! { <span role="status" class="theme-error">{message}</span> })}
    }
}
