import tkinter as tk
from tkinter import colorchooser
import colorsys
import re

def hex_to_rgb(hex_code):
    """Convertit un code hexadécimal en valeurs RGB de 0 à 1."""
    hex_code = hex_code.lstrip('#')
    return tuple(int(hex_code[i:i+2], 16) / 255.0 for i in (0, 2, 4))

def rgb_to_hex(r, g, b):
    """Convertit des valeurs RGB (0 à 1) en code hexadécimal."""
    return '#{:02x}{:02x}{:02x}'.format(int(r * 255), int(g * 255), int(b * 255))

def generate_palette(base_hex):
    """Génère la palette de 8 couleurs."""
    r, g, b = hex_to_rgb(base_hex)
    h, l, s = colorsys.rgb_to_hls(r, g, b)
    
    clamp = lambda x: max(0.0, min(1.0, x))
    
    c1 = base_hex
    l_dark = clamp(l * 0.40)
    l_light = clamp(l + (1 - l) * 0.50)
    
    c2 = rgb_to_hex(*colorsys.hls_to_rgb(h, l_dark, s))
    c3 = rgb_to_hex(*colorsys.hls_to_rgb(h, l_light, s))
    
    h_sec = (h + (137.5 / 360.0)) % 1.0
    c4 = rgb_to_hex(*colorsys.hls_to_rgb(h_sec, l, s))
    c5 = rgb_to_hex(*colorsys.hls_to_rgb(h_sec, l_dark, s))
    c6 = rgb_to_hex(*colorsys.hls_to_rgb(h_sec, l_light, s))
    
    c7 = rgb_to_hex(*colorsys.hls_to_rgb(h, 0.95, clamp(s * 0.15))) 
    c8 = rgb_to_hex(*colorsys.hls_to_rgb(h, 0.15, clamp(s * 0.15)))
    
    return [
        ("Base", c1),
        ("Base -60%", c2),
        ("Base +50%", c3),
        ("Sec. (137.5°)", c4),
        ("Sec. -60%", c5),
        ("Sec. +50%", c6),
        ("Fond Neutre", c7),
        ("Texte Neutre", c8)
    ]

class PaletteApp:
    def __init__(self, root):
        self.root = root
        self.root.title("GoldenHue - Générateur de Palette")
        self.root.geometry("900x420")
        self.root.configure(bg="#2c3e50")
        self.hex_var = tk.StringVar(value="#F1C40F")
        
        # En-tête / Titre
        self.title_lbl = tk.Label(
            root, text="GoldenHue", font=("Arial", 18, "bold"), fg="#f1c40f", bg="#2c3e50"
        )
        self.title_lbl.pack(pady=(15, 5))
        
        # Bouton principal
        self.btn = tk.Button(
            root, text="Choisir une couleur de base", 
            command=self.pick_color, 
            font=("Arial", 12, "bold"), bg="#ecf0f1", fg="#2c3e50", pady=8, cursor="hand2"
        )
        self.btn.pack(pady=10)

        # Saisie directe d'un code hexadecimal (ex. #3498DB).
        hex_controls = tk.Frame(root, bg="#2c3e50")
        hex_controls.pack(pady=(0, 5))
        tk.Label(hex_controls, text="Code HEX :", font=("Arial", 11, "bold"), fg="#ecf0f1", bg="#2c3e50").pack(side=tk.LEFT, padx=(0, 8))
        self.hex_entry = tk.Entry(hex_controls, textvariable=self.hex_var, width=12, justify="center", font=("Consolas", 13, "bold"), bg="#ecf0f1", fg="#2c3e50", relief="flat")
        self.hex_entry.pack(side=tk.LEFT, ipady=5)
        self.hex_entry.bind("<Return>", self.apply_hex_entry)
        self.hex_entry.bind("<FocusOut>", self.apply_hex_entry)
        tk.Button(hex_controls, text="Appliquer", command=self.apply_hex_entry, font=("Arial", 10, "bold"), bg="#f1c40f", fg="#2c3e50", relief="flat", padx=12, pady=5, cursor="hand2").pack(side=tk.LEFT, padx=8)
        
        # Cadre pour afficher les couleurs
        self.palette_frame = tk.Frame(root, bg="#2c3e50")
        self.palette_frame.pack(fill=tk.BOTH, expand=True, padx=20, pady=10)

        # Message de statut (Notification lors du clic)
        self.status_lbl = tk.Label(
            root, text="Cliquez sur une couleur pour copier son code HEX", 
            font=("Arial", 11, "italic"), fg="#bdc3c7", bg="#2c3e50", pady=10
        )
        self.status_lbl.pack()
        self.display_palette(self.hex_var.get())

    def pick_color(self):
        color = colorchooser.askcolor(title="Choisissez une couleur de base")
        if color[1]: 
            self.hex_var.set(color[1].upper())
            self.display_palette(color[1])

    def apply_hex_entry(self, _event=None):
        """Valide le champ #RRGGBB puis met a jour la palette sans ouvrir de dialogue."""
        value = self.hex_var.get().strip()
        if not value.startswith("#"):
            value = "#" + value
        if not re.fullmatch(r"#[0-9A-Fa-f]{6}", value):
            self.status_lbl.config(text="Code invalide : utilisez le format #RRGGBB.", fg="#e74c3c")
            self.hex_entry.focus_set()
            return
        value = value.upper()
        self.hex_var.set(value)
        self.display_palette(value)

    def copy_to_clipboard(self, hex_code):
        """Copie le code HEX dans le presse-papier et affiche un message de confirmation."""
        self.root.clipboard_clear()
        self.root.clipboard_append(hex_code.upper())
        self.root.update()
        
        # Mise à jour du message de confirmation
        self.status_lbl.config(
            text=f"✓ Code {hex_code.upper()} copié dans le presse-papier !", 
            fg="#2ecc71" # Vert
        )

    def display_palette(self, base_hex):
        # Nettoyer l'affichage précédent
        for widget in self.palette_frame.winfo_children():
            widget.destroy()
            
        colors = generate_palette(base_hex)
        self.status_lbl.config(
            text="Cliquez sur une couleur pour copier son code HEX", 
            fg="#bdc3c7"
        )
        
        # Afficher chaque couleur sous forme de bloc
        for name, hex_code in colors:
            color_box = tk.Frame(self.palette_frame, bg=hex_code, cursor="hand2")
            color_box.pack(side=tk.LEFT, fill=tk.BOTH, expand=True, padx=2)
            
            r, g, b = hex_to_rgb(hex_code)
            h, l, s = colorsys.rgb_to_hls(r, g, b)
            text_color = "white" if l < 0.5 else "black"
            
            lbl_name = tk.Label(
                color_box, text=name, bg=hex_code, fg=text_color, 
                font=("Arial", 9, "bold"), wraplength=80, cursor="hand2"
            )
            lbl_name.pack(expand=True)
            
            lbl_hex = tk.Label(
                color_box, text=hex_code.upper(), bg=hex_code, fg=text_color, 
                font=("Arial", 9), cursor="hand2"
            )
            lbl_hex.pack(expand=True)
            
            # Binding de l'événement de clic sur la boîte et les labels
            for element in (color_box, lbl_name, lbl_hex):
                element.bind("<Button-1>", lambda event, code=hex_code: self.copy_to_clipboard(code))

if __name__ == "__main__":
    root = tk.Tk()
    app = PaletteApp(root)
    root.mainloop()
