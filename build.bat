@echo off
REM Compile GoldenHue en application Windows sans console, avec son icone.
py -m PyInstaller --noconfirm --onefile --windowed --icon "GoldenHue.ico" --name "GoldenHue" "GoldenHue_App.py"
