import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from goldenhue import contrast_utils as ctr


class TestContrast(unittest.TestCase):
    def test_black_white_max_contrast(self):
        ratio = ctr.contrast_ratio("#000000", "#FFFFFF")
        self.assertAlmostEqual(ratio, 21.0, delta=0.01)

    def test_same_color_min_contrast(self):
        ratio = ctr.contrast_ratio("#3498DB", "#3498DB")
        self.assertAlmostEqual(ratio, 1.0, delta=0.01)

    def test_evaluate_contrast_badges(self):
        result = ctr.evaluate_contrast("#000000", "#FFFFFF")
        self.assertTrue(result.aaa_normal)
        self.assertEqual(result.badge, "AAA")

    def test_best_contrasting_bw(self):
        self.assertEqual(ctr.best_contrasting_bw("#000000"), "#FFFFFF")
        self.assertEqual(ctr.best_contrasting_bw("#FFFFFF"), "#000000")


if __name__ == "__main__":
    unittest.main()
