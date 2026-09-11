"""Protocol tests need only Python's standard library."""
import importlib.util
from pathlib import Path
import unittest

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


if __name__ == '__main__':
    unittest.main()
