"""
GoldenHue_App.py
=================
Point d'entrée de l'application. Lance l'interface Tkinter.
"""

import tkinter as tk

from goldenhue.resources import set_app_icon
from goldenhue.ui import PaletteApp

if __name__ == "__main__":
    root = tk.Tk()
    set_app_icon(root)  # posée au plus tôt pour éviter un flash de l'icône par défaut
    app = PaletteApp(root)
    root.mainloop()
