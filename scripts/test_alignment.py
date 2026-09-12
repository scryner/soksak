"""Protocol tests need only Python's standard library."""
import importlib.util
from pathlib import Path
import unittest
import json
import tempfile
from types import SimpleNamespace

spec = importlib.util.spec_from_file_location('alignment', Path(__file__).resolve().parents[1] / 'soksak-lib/src/transcribe/alignment/align.py')
alignment = importlib.util.module_from_spec(spec)
spec.loader.exec_module(alignment)


class AlignmentTests(unittest.TestCase):
    def result(self, chars):
        return {'segments': [{'chars': chars}]}

    def char(self, text, start=1.0, end=1.1, score=0.9):
        return {'char': text, 'start': start, 'end': end, 'score': score}

    def test_unicode_text_and_punctuation_do_not_need_rewriting(self):
        result = self.result([self.char('안'), self.char('녕', 1.2, 1.4), {'char': '!'}])
        candidate = alignment.candidate_from_result(result, ' 안녕! ')
        self.assertEqual(candidate['start'], 1.0)
        self.assertEqual(candidate['end'], 1.4)
        self.assertEqual(candidate['coverage'], 1.0)

    def test_partial_alignment_must_not_trim_unaligned_edges(self):
        result = self.result([{'char': '2'}, self.char('년')])
        self.assertEqual(alignment.candidate_from_result(result, '2년')['reason'], 'unaligned_edge')

    def test_character_mismatch_is_rejected(self):
        result = self.result([self.char('a')])
        self.assertEqual(alignment.candidate_from_result(result, 'abc')['reason'], 'character_mismatch')

    def test_nonfinite_edges_are_rejected(self):
        result = self.result([self.char('a', end=float('nan'))])
        self.assertEqual(alignment.candidate_from_result(result, 'a')['reason'], 'unaligned_edge')

    def devices(self, mps=False, cuda=False):
        return SimpleNamespace(cuda=SimpleNamespace(is_available=lambda: cuda),
                               backends=SimpleNamespace(mps=SimpleNamespace(is_available=lambda: mps)))

    def test_auto_device_uses_acceleration_and_explicit_cpu_stays_on_cpu(self):
        self.assertEqual(alignment.select_device(self.devices(mps=True), 'auto'), 'mps')
        self.assertEqual(alignment.select_device(self.devices(cuda=True), 'auto'), 'cuda')
        self.assertEqual(alignment.select_device(self.devices(), 'auto'), 'cpu')
        self.assertEqual(alignment.select_device(self.devices(mps=True), 'cpu'), 'cpu')

    def test_unavailable_explicit_device_and_invalid_device_are_reported(self):
        with self.assertRaises(RuntimeError):
            alignment.select_device(self.devices(), 'mps')
        with self.assertRaises(ValueError):
            alignment.select_device(self.devices(), 'typo')

    def test_progress_file_contains_latest_complete_update(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'progress.json'
            request = {'progress': str(path)}
            alignment.write_progress(request, 'loading_model', 0)
            alignment.write_progress(request, 'aligning', 7)
            self.assertEqual(json.loads(path.read_text()), {'phase': 'aligning', 'completed': 7})
            self.assertFalse(path.with_suffix('.tmp').exists())

    def test_accelerator_failure_retries_on_cpu_and_reports_the_reason(self):
        calls = []
        model = SimpleNamespace(to=lambda device: calls.append(('transfer', device)))
        def align(*args, **kwargs):
            device = args[4]
            calls.append(('align', device))
            if device == 'mps':
                raise RuntimeError('unsupported operator')
            return {'segments': []}
        result, device, reason = alignment.align_with_fallback(
            align, [], model, {}, [], 'mps', 'auto', self.devices())
        self.assertEqual(result, {'segments': []})
        self.assertEqual(device, 'cpu')
        self.assertIn('unsupported operator', reason)
        self.assertEqual(calls, [('align', 'mps'), ('transfer', 'cpu'), ('align', 'cpu')])

    def test_explicit_accelerator_failure_is_not_silently_retried(self):
        def align(*args, **kwargs):
            raise RuntimeError('unsupported operator')
        with self.assertRaises(RuntimeError):
            alignment.align_with_fallback(align, [], None, {}, [], 'mps', 'mps', self.devices())


if __name__ == '__main__':
    unittest.main()
