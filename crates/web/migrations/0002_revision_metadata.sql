ALTER TABLE revisions ADD COLUMN seed TEXT NOT NULL DEFAULT '';
ALTER TABLE revisions ADD COLUMN entry_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE revisions ADD COLUMN stage_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE revisions ADD COLUMN word_order TEXT NOT NULL DEFAULT '';
ALTER TABLE reviews ADD COLUMN streak INTEGER NOT NULL DEFAULT 0;
ALTER TABLE reviews ADD COLUMN next_review INTEGER NOT NULL DEFAULT 0;
UPDATE revisions SET seed=json_extract(package,'$.recipe.seed'),entry_count=json_array_length(package,'$.lexicon'),stage_count=json_array_length(package,'$.stages'),word_order=json_extract(package,'$.grammar.order');
