from __future__ import annotations

import argparse
import json
from pathlib import Path

import torch
from train import BOS, EOS, Config, Translator, encode


def translate(directory: Path, revision: str, direction: str, text: str) -> str:
    manifest = json.loads((directory / 'manifest.json').read_text())
    if revision != manifest['revision']:
        raise ValueError('The model belongs to a different language revision')
    config = Config(**manifest['config'])
    model = Translator(config)
    model.load_state_dict(torch.load(directory / 'best.pt', map_location='cpu', weights_only=True))
    model.eval()
    source = torch.tensor([encode(text, direction, config.maximum)])
    target = torch.tensor([[BOS]])
    output = []
    with torch.inference_mode():
        for _ in range(config.maximum - 1):
            token = int(model(source, target)[0, -1].argmax())
            if token == EOS:
                return bytes(output).decode('utf-8', errors='strict')
            if token < 5:
                raise ValueError('The experimental model produced an invalid control token')
            output.append(token - 5)
            target = torch.cat((target, torch.tensor([[token]])), dim=1)
    raise ValueError('The model did not terminate within its output budget')


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument('model', type=Path)
    parser.add_argument('--revision', required=True)
    parser.add_argument('--direction', choices=['to_english', 'from_english'], required=True)
    parser.add_argument('text')
    args = parser.parse_args()
    print(json.dumps({'output': translate(args.model, args.revision, args.direction, args.text), 'verified': False}))


if __name__ == '__main__':
    main()
