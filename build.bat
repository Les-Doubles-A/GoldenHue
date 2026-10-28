@echo off
<<<<<<< HEAD
chcp 65001 >nul
setlocal enabledelayedexpansion
title GoldenHue - Compilation .exe
cd /d "%~dp0"

echo ============================================
echo   GoldenHue - Compilation en .exe (Windows)
echo ============================================
echo.

REM --- 1. Verifier que Python est installe ---
where python >nul 2>nul
if errorlevel 1 (
    echo [ERREUR] Python n'est pas trouve dans le PATH.
    echo Installez Python depuis https://www.python.org/downloads/
    echo puis cochez "Add Python to PATH" lors de l'installation.
    pause
    exit /b 1
)

echo [1/5] Python detecte : 
python --version
echo.

REM --- 2. Creer un environnement virtuel dedie a la compilation ---
if not exist "build_venv" (
    echo [2/5] Creation de l'environnement virtuel...
    python -m venv build_venv
) else (
    echo [2/5] Environnement virtuel deja present.
)

call build_venv\Scripts\activate.bat
echo.

REM --- 3. Installer les dependances + PyInstaller ---
echo [3/5] Installation des dependances (Pillow, numpy, PyInstaller)...
python -m pip install --upgrade pip >nul
pip install -r requirements.txt
pip install pyinstaller
echo.

REM --- 4. Nettoyer les anciens builds ---
echo [4/5] Nettoyage des anciens fichiers de compilation...
if exist "build" rmdir /s /q "build"
if exist "dist" rmdir /s /q "dist"
if exist "GoldenHue.spec" del /q "GoldenHue.spec"
echo.

REM --- 5. Compiler l'application avec l'icone incluse ---
echo [5/5] Compilation en .exe (icone incluse, fichier unique, sans console)...
pyinstaller --noconfirm --onefile --windowed ^
    --name "GoldenHue" ^
    --icon "GoldenHue.ico" ^
    --add-data "GoldenHue.ico;." ^
    --add-data "GoldenHue.png;." ^
    GoldenHue_App.py

echo.

if exist "dist\GoldenHue.exe" (
    copy /y "dist\GoldenHue.exe" "GoldenHue.exe" >nul
    echo ============================================
    echo   TERMINE : GoldenHue.exe cree avec succes !
    echo   Fichier disponible ici : %cd%\GoldenHue.exe
    echo ============================================
) else (
    echo [ERREUR] La compilation a echoue. Voir les messages ci-dessus.
)

call build_venv\Scripts\deactivate.bat
echo.
pause
=======
REM Compile GoldenHue en application Windows sans console, avec son icone.
py -m PyInstaller --noconfirm --onefile --windowed --icon "GoldenHue.ico" --name "GoldenHue" "GoldenHue_App.py"
>>>>>>> d0a94e02c72f98998c2d55a7803e4292a693c876
