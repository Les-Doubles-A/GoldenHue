import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from goldenhue import export_utils as exu


SAMPLE = [("Base", "#F1C40F"), ("Complément", "#0F4FF1")]


class TestExport(unittest.TestCase):
    def setUp(self):
        self.manager = exu.ExportManager(SAMPLE, "TestPalette")

    def test_csv_has_header_and_rows(self):
        csv_text = self.manager.to_csv()
        self.assertIn("role,hex,rgb,hsl,hsv", csv_text)
        self.assertIn("#F1C40F", csv_text)

    def test_json_structure(self):
        import json
        data = json.loads(self.manager.to_json())
        self.assertEqual(data["palette"], "TestPalette")
        self.assertEqual(len(data["colors"]), 2)
        self.assertEqual(data["colors"][0]["hex"], "#F1C40F")

    def test_css_variables(self):
        css = self.manager.to_css()
        self.assertIn(":root {", css)
        self.assertIn("--base: #F1C40F;", css)

    def test_scss_variables(self):
        scss = self.manager.to_scss()
        self.assertIn("$goldenhue-base: #F1C40F;", scss)

    def test_txt(self):
        txt = self.manager.to_txt()
        self.assertIn("Base : #F1C40F", txt)

    def test_ase_bytes_header(self):
        data = self.manager.to_ase_bytes()
        self.assertEqual(data[:4], b"ASEF")

    def test_png_bytes_is_valid_png(self):
        data = self.manager.to_png_bytes()
        self.assertEqual(data[:8], b"\x89PNG\r\n\x1a\n")


if __name__ == "__main__":
    unittest.main()
