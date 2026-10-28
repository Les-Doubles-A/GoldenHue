import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from goldenhue import color_utils as cu


class TestConversions(unittest.TestCase):
    def test_hex_rgb_roundtrip(self):
        for hexcode in ["#000000", "#FFFFFF", "#F1C40F", "#3498DB", "#1A2B3C"]:
            r, g, b = cu.hex_to_rgb256(hexcode)
            self.assertEqual(cu.rgb256_to_hex(r, g, b), hexcode)

    def test_hsl_roundtrip_approx(self):
        hexcode = "#3498DB"
        h, s, l = cu.hex_to_hsl(hexcode)
        back = cu.hsl_to_hex(h, s, l)
        self.assertEqual(back, hexcode)

    def test_normalize_hex(self):
        self.assertEqual(cu.normalize_hex("f1c40f"), "#F1C40F")
        self.assertEqual(cu.normalize_hex("#f1c40f"), "#F1C40F")
        with self.assertRaises(ValueError):
            cu.normalize_hex("not-a-color")

    def test_format_color(self):
        self.assertTrue(cu.format_color("#3498DB", "RGB").startswith("rgb("))
        self.assertTrue(cu.format_color("#3498DB", "HSL").startswith("hsl("))
        self.assertTrue(cu.format_color("#3498DB", "HSV").startswith("hsv("))
        self.assertEqual(cu.format_color("#3498db", "HEX"), "#3498DB")


class TestSchemes(unittest.TestCase):
    def test_complementary_hue_is_opposite(self):
        swatches = cu.generate_complementary("#FF0000")
        base = next(s for s in swatches if s.role == "Base")
        comp = next(s for s in swatches if s.role == "Complément")
        h_base, _, _ = cu.hex_to_hsl(base.hex)
        h_comp, _, _ = cu.hex_to_hsl(comp.hex)
        diff = abs((h_comp - h_base + 180) % 360 - 180)
        self.assertAlmostEqual(abs(h_comp - h_base) % 360, 180, delta=1.0)

    def test_all_schemes_have_neutrals(self):
        for scheme in cu.SchemeType:
            swatches = cu.generate_palette("#2ECC71", scheme)
            roles = [s.role for s in swatches]
            self.assertIn(cu.NEUTRAL_ROLE_BG, roles)
            self.assertIn(cu.NEUTRAL_ROLE_TEXT, roles)

    def test_tetradic_has_four_hues(self):
        swatches = cu.generate_tetradic("#2ECC71")
        # 4 teintes * 3 tons + 2 neutres = 14
        self.assertEqual(len(swatches), 14)

    def test_generate_all_schemes_keys(self):
        result = cu.generate_all_schemes("#2ECC71")
        self.assertEqual(set(result.keys()), set(cu.SchemeType))

    def test_random_hex_is_valid(self):
        for _ in range(20):
            self.assertTrue(cu.is_valid_hex(cu.random_hex()))


if __name__ == "__main__":
    unittest.main()
