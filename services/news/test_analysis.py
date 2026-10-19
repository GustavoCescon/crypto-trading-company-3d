import unittest
from analysis import analyze, fingerprint, validate_ai


class AnalysisTests(unittest.TestCase):
    def test_entities_and_sentiment(self):
        result = analyze("Bitcoin rally follows ETF approval")
        self.assertEqual(result["assets"], ["BTC"])
        self.assertGreater(result["sentiment"], 0)
        self.assertFalse(result["can_place_orders"])

    def test_dedup_normalizes_punctuation(self):
        self.assertEqual(fingerprint("Bitcoin: Rally!"), fingerprint("bitcoin rally"))

    def test_negation(self):
        self.assertLess(analyze("Bitcoin not approved")["sentiment"], 0)

    def test_untrusted_model_output(self):
        with self.assertRaises(ValueError):
            validate_ai({"sentiment": 300}, analyze("BTC"), "test")
        with self.assertRaises(ValueError):
            validate_ai({"sentiment": "nan"}, analyze("BTC"), "test")
