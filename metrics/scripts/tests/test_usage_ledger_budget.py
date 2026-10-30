import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.dirname(__file__)))
import usage_ledger as L


class Pricing(unittest.TestCase):
    def test_inconsistent_split_is_range(self):
        L.ISSUES.clear()
        u = dict(input_tokens=0, output_tokens=0, cache_read_input_tokens=0,
                 cache_creation_input_tokens=0, ephemeral_1h_input_tokens=100)
        low, high, _ = L.native_usd(u, 'claude-opus-5-5')
        self.assertEqual((low, high), (0, .0008))
        self.assertEqual(L.ISSUES[-1]['kind'], 'inconsistent_cache_split')

    def test_haiku_boundary(self):
        u = dict(input_tokens=100000, output_tokens=0, cache_read_input_tokens=0, cache_creation_input_tokens=0)
        self.assertAlmostEqual(L.native_usd(u, 'claude-haiku-5-5')[0], .01)
        u['input_tokens'] += 1
        self.assertAlmostEqual(L.native_usd(u, 'claude-haiku-5-5')[0], .0500005)
