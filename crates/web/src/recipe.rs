use etyloom_core::{ENGINE, LEGACY_ENGINE, Recipe};
use leptos::prelude::*;

fn parse(source: &str) -> Result<Recipe, String> {
    let recipe: Recipe = serde_json::from_str(source).map_err(|error| error.to_string())?;
    recipe.validate().map_err(|error| error.to_string())?;
    Ok(recipe)
}

fn status(recipe: &Recipe, saved: Option<&Recipe>) -> (&'static str, &'static str) {
    if saved.is_some_and(|saved| saved != recipe) {
        return (
            "Recipe changes not applied",
            "Only this editor has changed. Generate a new draft to apply the recipe. The saved language and its exports are unchanged.",
        );
    }
    if recipe.engine == LEGACY_ENGINE {
        return (
            "Older generator selected",
            "This recipe will replay its original generator. Upgrade it to prepare a new result; your saved language is not changed by this button.",
        );
    }
    if saved.is_some() {
        (
            "Generator up to date",
            "This saved language already uses the current generator. No upgrade is needed.",
        )
    } else {
        (
            "Current generator selected",
            "This recipe will use the current generator when you generate a language.",
        )
    }
}

#[component]
pub fn GeneratorStatus(
    source: RwSignal<String>,
    error: RwSignal<Option<String>>,
    #[prop(into)] disabled: Signal<bool>,
    #[prop(optional)] saved: Option<Recipe>,
) -> impl IntoView {
    let saved = StoredValue::new(saved);
    let parsed = Memo::new(move |_| {
        let source = source.get();
        (!source.trim().is_empty()).then(|| parse(&source))
    });
    let upgrade = move |_| {
        let result = (|| -> Result<String, String> {
            let mut recipe = parse(&source.get_untracked())?;
            recipe.upgrade().map_err(|error| error.to_string())?;
            serde_json::to_string_pretty(&recipe).map_err(|error| error.to_string())
        })();
        match result {
            Ok(value) => {
                source.set(value);
                error.set(None);
            }
            Err(message) => error.set(Some(message)),
        }
    };
    view! {
        {move || parsed.get().map(|result| match result {
            Err(message) => view! {
                <p class="field-help" role="alert">"Cannot determine generator: "{message}</p>
            }.into_any(),
            Ok(recipe) => {
                let saved = saved.get_value();
                let (title, message) = status(&recipe, saved.as_ref());
                let legacy = recipe.engine == LEGACY_ENGINE;
                view! {
                    <section class="generator-status my-4 border-y border-line py-4" aria-label="Generator status">
                        <div role="status" aria-live="polite">
                            <strong>{title}</strong>
                            <dl class="forms-list">
                                {saved.map(|saved| view! { <div><dt>"Saved language"</dt><dd><code data-testid="saved-engine">{saved.engine}</code></dd></div> })}
                                <div><dt>"Next generation"</dt><dd><code data-testid="recipe-engine">{recipe.engine}</code></dd></div>
                                <div><dt>"Current generator"</dt><dd><code>{ENGINE}</code></dd></div>
                            </dl>
                            <p class="field-help">{message}</p>
                        </div>
                        {legacy.then(|| view! {
                            <button class="button secondary small-button" type="button" disabled=move || disabled.get() on:click=upgrade>"Upgrade generator"</button>
                        })}
                    </section>
                }.into_any()
            }
        })}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_recipe_is_current() {
        let recipe = Recipe::default();
        assert_eq!(recipe.engine, ENGINE);
        assert_eq!(status(&recipe, Some(&recipe)).0, "Generator up to date");
        assert_eq!(status(&recipe, None).0, "Current generator selected");
    }

    #[test]
    fn upgrade_only_changes_the_editor() -> etyloom_core::Result<()> {
        let saved = Recipe {
            engine: LEGACY_ENGINE.into(),
            ..Recipe::default()
        };
        let mut editor = saved.clone();
        assert_eq!(status(&editor, Some(&saved)).0, "Older generator selected");
        editor.upgrade()?;
        assert_eq!(saved.engine, LEGACY_ENGINE);
        assert_eq!(editor.engine, ENGINE);
        assert_eq!(
            status(&editor, Some(&saved)).0,
            "Recipe changes not applied"
        );
        assert_eq!(status(&editor, Some(&editor)).0, "Generator up to date");
        Ok(())
    }

    #[test]
    fn other_recipe_edits_are_also_pending() {
        let saved = Recipe::default();
        let mut editor = saved.clone();
        editor.seed = "a-new-seed".into();
        assert_eq!(
            status(&editor, Some(&saved)).0,
            "Recipe changes not applied"
        );
    }

    #[test]
    fn invalid_or_unsupported_recipes_cannot_appear_current() {
        for source in ["", "{", r#"{"engine":"etyloom/99.0.0"}"#, r#"{"seed":""}"#] {
            assert!(parse(source).is_err());
        }
    }
}
