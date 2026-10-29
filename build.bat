@echo off
chcp 65001 >nul
setlocal enabledelayedexpansion
title GoldenHue - Compilation Rust Release (.exe)
cd /d "%~dp0"

echo ==================================================
echo   GoldenHue - Compilation Rust Optimisee (.exe)
echo ==================================================
echo.

where cargo >nul 2>nul
if errorlevel 1 (
    echo [ERREUR] Cargo / Rust n'est pas trouve dans le PATH.
    echo Installez Rust depuis https://rustup.rs/
    pause
    exit /b 1
)

echo [1/3] Toolchain Rust detectee :
rustc --version
cargo --version
echo.

echo [2/3] Execution des tests unitaires...
cargo test --offline
if errorlevel 1 (
    echo [ERREUR] Les tests unitaires ont echoue.
    pause
    exit /b 1
)
echo.

echo [3/3] Compilation en mode Release ultra-optimise (LTO + strip + opt-level=3)...
cargo build --release --offline
if errorlevel 1 (
    echo [ERREUR] La compilation a echoue.
    pause
    exit /b 1
)

if exist "target\release\GoldenHue.exe" (
    copy /y "target\release\GoldenHue.exe" "GoldenHue.exe" >nul
    echo.
    echo ==================================================
    echo   TERMINE : GoldenHue.exe cree avec succes !
    echo   Fichier disponible ici : %cd%\GoldenHue.exe
    echo ==================================================
) else (
    echo [ERREUR] Binaire introuvable dans target\release\GoldenHue.exe.
)

echo.
pause
