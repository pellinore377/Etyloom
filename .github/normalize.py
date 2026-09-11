from pathlib import Path

p = Path('crates/engine/src/grammar.rs')
s = p.read_text()
s = s.replace('matches!(\n            self.package.grammar.order,\n            Order::Svo | Order::Vso | Order::Vos\n        )', '!self.package.grammar.negation_after')
p.write_text(s)
p = Path('crates/web/src/workspace.rs')
p.write_text(p.read_text().replace('Result<Recipe, String>', 'std::result::Result<Recipe, String>'))
p = Path('crates/web/src/client.rs')
p.write_text(p.read_text().replace('use etyloom_core::ApiError;', '#[cfg(feature = "hydrate")]\nuse etyloom_core::ApiError;'))
p = Path('style/app.css')
p.write_text(p.read_text().replace('.workspace-nav > a:first-child,.theme-label,.account { display: none; }', '.workspace-nav > a:first-child,.theme-label { display: none; }\n  .account { padding-left: 10px; font-size: 11px; }'))
