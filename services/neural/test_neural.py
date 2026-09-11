import json
import tempfile
import unittest
from pathlib import Path

import torch
from train import Config, Translator, collate, encode, loss_epoch, read_corpus


class NeuralTests(unittest.TestCase):
    def setUp(self):
        torch.manual_seed(3)
        torch.set_num_threads(1)

    def test_unicode_bytes_and_length(self):
        value = encode('šæŋ öü', None, 64)
        self.assertEqual(bytes(token - 5 for token in value[1:-1]).decode(), 'šæŋ öü')
        with self.assertRaises(ValueError):
            encode('x' * 65, None, 64)

    def test_training_step_changes_parameters(self):
        model = Translator(Config(width=16, heads=4, layers=1, maximum=64, dropout=0))
        source, target = collate([(encode('I walk', 'from_english', 64), encode('ner tal', None, 64))])
        before = model.output.weight.detach().clone()
        optimizer = torch.optim.AdamW(model.parameters(), lr=0.001)
        loss = loss_epoch(model, [(source, target)], torch.device('cpu'), optimizer)
        self.assertGreater(loss, 0)
        self.assertFalse(torch.equal(before, model.output.weight))

    def test_cross_split_meaning_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'data.jsonl'
            rows = [{'revision': 'test', 'split': split, 'direction': 'from_english', 'source': 'I walk', 'target': 'ner tal', 'meaning': {'same': True}} for split in ('train', 'validation', 'test')]
            path.write_text('\n'.join(json.dumps(row) for row in rows))
            with self.assertRaisesRegex(ValueError, 'crosses data splits'):
                read_corpus(path)


if __name__ == '__main__':
    unittest.main()
