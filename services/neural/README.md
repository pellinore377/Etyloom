# Optional local neural baseline

This is an **experimental, runnable byte-level Transformer baseline**, not a trained translator and not part of the web request path. It trains both directions using the engine's JSONL exports. No external model, dictionary, or API is downloaded at training or inference time. The Docker build downloads the pinned PyTorch runtime.

The baseline starts from scratch so it can run without downloaded model weights. Comparing it to an appropriately licensed pretrained sequence-to-sequence model is a subsequent experiment, not a demonstrated result here.

## Train

Export a corpus from the language's Translate page, or run:

```sh
etyloom corpus language.json 50000 > corpus.jsonl
python -m venv .venv
. .venv/bin/activate
pip install torch==2.10.0 --index-url https://download.pytorch.org/whl/cpu
python train.py corpus.jsonl ./model-run --epochs 30 --batch-size 64
```

For GPU work install a PyTorch build compatible with your driver/hardware and use `--device cuda`. A requested but unavailable GPU is an error, not a silent CPU fallback.

The trainer records the language revision, corpus checksum, model settings, framework version, split sizes and losses. It rejects mixed language revisions, meaning leakage across splits, invalid dimensions, expressions that exceed its byte budget, and existing output directories. It saves the best validation checkpoint and stops after five non-improving validation epochs. Test data is not used to select checkpoints.

## Infer

```sh
python infer.py ./model-run --revision FULL_LANGUAGE_REVISION --direction from_english 'I see the river'
python infer.py ./model-run --revision FULL_LANGUAGE_REVISION --direction to_english 'generated expression'
```

Outputs are explicitly marked `verified: false`. Wrong-revision requests, invalid byte sequences, special-token failures and unterminated output fail explicitly. The engine remains authoritative; this script is not connected to production translation or answer grading.

Only load checkpoints you created or trust. `weights_only=True` restricts deserialization; it is not a sandbox against malicious or oversized tensor files.

## Docker

From the repository root, place the exported corpus at `training/corpus.jsonl`, then:

```sh
docker compose -f compose.neural.yaml run --rm trainer /data/corpus.jsonl /models/run-01 --epochs 30
```

The default optional container is CPU-only and has no network at runtime. It is independent of the application and does not have the OIDC secret or application database. Use a fresh output directory for each run.

## What the metrics mean

Training/validation/test cross-entropy is useful for debugging and checkpoint selection. It does **not** establish translation quality, semantic preservation, grammaticality, vocabulary coverage, or readiness for unrestricted English. Before adoption, evaluate independently authored test sentences, held-out lexical combinations, negation and tense preservation, reverse-direction accuracy, ambiguity, invalid outputs and human acceptability.

Unit tests cover UTF-8 representation, an actual parameter-updating training step, and split-leakage rejection. They are not fluency tests. No trained language model or accuracy claim ships with this release.
