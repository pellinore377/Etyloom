from __future__ import annotations

import argparse
import hashlib
import json
import math
import random
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

import torch
from torch import Tensor, nn
from torch.utils.data import DataLoader, Dataset

PAD, BOS, EOS, TO_ENGLISH, FROM_ENGLISH = range(5)
VOCAB = 261


@dataclass(frozen=True)
class Config:
    width: int = 128
    heads: int = 4
    layers: int = 3
    maximum: int = 256
    dropout: float = 0.1


class Translator(nn.Module):
    def __init__(self, config: Config) -> None:
        super().__init__()
        self.config = config
        self.tokens = nn.Embedding(VOCAB, config.width, padding_idx=PAD)
        self.positions = nn.Embedding(config.maximum, config.width)
        self.transformer = nn.Transformer(
            d_model=config.width, nhead=config.heads,
            num_encoder_layers=config.layers, num_decoder_layers=config.layers,
            dim_feedforward=config.width * 4, dropout=config.dropout,
            batch_first=True,
        )
        self.output = nn.Linear(config.width, VOCAB)

    def embed(self, tokens: Tensor) -> Tensor:
        positions = torch.arange(tokens.shape[1], device=tokens.device)
        return self.tokens(tokens) * math.sqrt(self.config.width) + self.positions(positions)

    def forward(self, source: Tensor, target: Tensor) -> Tensor:
        mask = torch.triu(torch.ones(target.shape[1], target.shape[1], device=target.device, dtype=torch.bool), 1)
        values = self.transformer(
            self.embed(source), self.embed(target), tgt_mask=mask,
            src_key_padding_mask=source.eq(PAD),
            tgt_key_padding_mask=target.eq(PAD),
            memory_key_padding_mask=source.eq(PAD),
        )
        return self.output(values)


def encode(text: str, direction: str | None, maximum: int) -> list[int]:
    prefix = [TO_ENGLISH if direction == 'to_english' else FROM_ENGLISH] if direction else [BOS]
    result = prefix + [byte + 5 for byte in text.encode('utf-8')] + [EOS]
    if len(result) > maximum:
        raise ValueError(f'Expression exceeds the {maximum}-byte model limit; it was not truncated')
    return result


class Pairs(Dataset):
    def __init__(self, rows: list[dict[str, Any]], maximum: int) -> None:
        self.items = [(encode(row['source'], row['direction'], maximum), encode(row['target'], None, maximum)) for row in rows]

    def __len__(self) -> int:
        return len(self.items)

    def __getitem__(self, index: int) -> tuple[list[int], list[int]]:
        return self.items[index]


def collate(items: list[tuple[list[int], list[int]]]) -> tuple[Tensor, Tensor]:
    def pad(values: list[list[int]]) -> Tensor:
        result = torch.full((len(values), max(map(len, values))), PAD, dtype=torch.long)
        for index, value in enumerate(values):
            result[index, :len(value)] = torch.tensor(value)
        return result
    return pad([item[0] for item in items]), pad([item[1] for item in items])


def read_corpus(path: Path) -> tuple[dict[str, list[dict[str, Any]]], str, str]:
    splits: dict[str, list[dict[str, Any]]] = {'train': [], 'validation': [], 'test': []}
    revisions: set[str] = set()
    assignments: dict[str, str] = {}
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        for number, line in enumerate(stream, 1):
            digest.update(line)
            row = json.loads(line)
            if row['split'] not in splits or row['direction'] not in ('to_english', 'from_english'):
                raise ValueError(f'Unsupported split or direction on line {number}')
            revisions.add(row['revision'])
            identity = hashlib.sha256(json.dumps(row['meaning'], sort_keys=True).encode()).hexdigest()
            previous = assignments.setdefault(identity, row['split'])
            if previous != row['split']:
                raise ValueError(f'The same meaning crosses data splits on line {number}')
            splits[row['split']].append(row)
    if len(revisions) != 1 or any(not values for values in splits.values()):
        raise ValueError('A corpus needs exactly one language revision and nonempty train, validation and test splits')
    return splits, next(iter(revisions)), digest.hexdigest()


def loss_epoch(model: Translator, batches: DataLoader, device: torch.device, optimizer: torch.optim.Optimizer | None) -> float:
    model.train(optimizer is not None)
    criterion = nn.CrossEntropyLoss(ignore_index=PAD, reduction='sum')
    total, tokens = 0.0, 0
    with torch.set_grad_enabled(optimizer is not None):
        for source, target in batches:
            source, target = source.to(device), target.to(device)
            if optimizer:
                optimizer.zero_grad(set_to_none=True)
            output = model(source, target[:, :-1])
            loss = criterion(output.reshape(-1, VOCAB), target[:, 1:].reshape(-1))
            count = int(target[:, 1:].ne(PAD).sum())
            if optimizer:
                (loss / max(count, 1)).backward()
                nn.utils.clip_grad_norm_(model.parameters(), 1.0)
                optimizer.step()
            total += float(loss.detach())
            tokens += count
    return total / max(tokens, 1)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument('corpus', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--epochs', type=int, default=30)
    parser.add_argument('--batch-size', type=int, default=64)
    parser.add_argument('--width', type=int, default=128)
    parser.add_argument('--layers', type=int, default=3)
    parser.add_argument('--maximum', type=int, default=256)
    parser.add_argument('--seed', type=int, default=42)
    parser.add_argument('--device', choices=['cpu', 'cuda'], default='cpu')
    args = parser.parse_args()
    if args.epochs < 1 or args.batch_size < 1 or args.width < 16 or args.width % 4 or not 1 <= args.layers <= 12 or not 32 <= args.maximum <= 1024:
        parser.error('Invalid training dimensions or budget')
    if args.output.exists():
        parser.error('Output directory already exists; training never overwrites an existing model')
    if args.device == 'cuda' and not torch.cuda.is_available():
        parser.error('CUDA was requested but is unavailable')
    random.seed(args.seed)
    torch.manual_seed(args.seed)
    splits, revision, digest = read_corpus(args.corpus)
    config = Config(width=args.width, layers=args.layers, maximum=args.maximum)
    generator = torch.Generator().manual_seed(args.seed)
    batches = {name: DataLoader(Pairs(rows, config.maximum), batch_size=args.batch_size, shuffle=name == 'train', collate_fn=collate, generator=generator) for name, rows in splits.items()}
    device = torch.device(args.device)
    model = Translator(config).to(device)
    optimizer = torch.optim.AdamW(model.parameters(), lr=3e-4)
    args.output.mkdir(parents=True)
    manifest = {'revision': revision, 'corpus_sha256': digest, 'config': asdict(config), 'seed': args.seed, 'torch': torch.__version__, 'status': 'experimental', 'split_rows': {key: len(value) for key, value in splits.items()}}
    (args.output / 'manifest.json').write_text(json.dumps(manifest, indent=2))
    best, stale = math.inf, 0
    with (args.output / 'metrics.jsonl').open('w') as metrics:
        for epoch in range(args.epochs):
            train = loss_epoch(model, batches['train'], device, optimizer)
            validation = loss_epoch(model, batches['validation'], device, None)
            record = {'epoch': epoch + 1, 'train_loss': train, 'validation_loss': validation}
            metrics.write(json.dumps(record) + '\n')
            metrics.flush()
            print(json.dumps(record), flush=True)
            if validation < best:
                best, stale = validation, 0
                torch.save(model.state_dict(), args.output / 'best.pt')
            else:
                stale += 1
                if stale >= 5:
                    break
    model.load_state_dict(torch.load(args.output / 'best.pt', map_location=device, weights_only=True))
    test_loss = loss_epoch(model, batches['test'], device, None)
    manifest.update(best_validation_loss=best, held_out_test_loss=test_loss)
    (args.output / 'manifest.json').write_text(json.dumps(manifest, indent=2))
    print(json.dumps({'test_loss': test_loss, 'revision': revision, 'status': 'experimental; translation accuracy not established'}))


if __name__ == '__main__':
    main()
