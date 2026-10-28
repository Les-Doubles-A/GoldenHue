"""
resources.py
=============
Localisation des fichiers de ressources (icône...) et application de
l'icône de l'application sur la fenêtre Tk, que le programme tourne
depuis les sources ou depuis un .exe compilé avec PyInstaller (--onefile).
"""

from __future__ import annotations

import os
import sys
import tkinter as tk


def resource_path(relative_path: str) -> str:
    """
    Retourne le chemin absolu vers une ressource embarquée.

    - En mode compilé (PyInstaller --onefile), les fichiers ajoutés via
      --add-data sont extraits dans un dossier temporaire pointé par
      sys._MEIPASS.
    - En mode source, on remonte à la racine du projet (parent du
      package goldenhue/), là où se trouvent GoldenHue.ico / .png.
    """
    base_path = getattr(sys, "_MEIPASS", None)
    if base_path is None:
        base_path = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    return os.path.join(base_path, relative_path)


def set_app_icon(root: tk.Tk | tk.Toplevel) -> None:
    """
    Applique l'icône GoldenHue à une fenêtre : barre de titre, barre des
    tâches (Windows) et Alt+Tab. Utilise à la fois .ico (natif Windows) et
    .png (iconphoto, nécessaire pour un rendu correct multi-plateforme et
    pour que le Alt+Tab / la barre des tâches reprennent bien l'icône).
    """
    ico_path = resource_path("GoldenHue.ico")
    png_path = resource_path("GoldenHue.png")

    try:
        if os.path.exists(ico_path):
            # `default=` propage aussi l'icône aux futures fenêtres Toplevel.
            root.iconbitmap(default=ico_path)
    except tk.TclError:
        pass

    try:
        if os.path.exists(png_path):
            icon_img = tk.PhotoImage(file=png_path)
            root.iconphoto(True, icon_img)
            # Garder une référence : sinon le PhotoImage est garbage-collecté
            # et l'icône disparaît.
            root._goldenhue_icon_ref = icon_img  # type: ignore[attr-defined]
    except tk.TclError:
        pass
