@echo off
title Orchestrator Monitor
color 0A

:loop
cls
echo ╔════════════════════════════════════════════════╗
echo ║       ORCHESTRATOR MONITOR                    ║
echo ╚════════════════════════════════════════════════╝
echo.
echo Time: %date% %time%
echo.

cd /d D:\SC\cs-orchestrator-multi-mawari

echo ─────────────────────────────────────────────────
echo STATE:
echo ─────────────────────────────────────────────────
if exist state.json (
    type state.json
) else (
    echo No state file found
)

echo.
echo ─────────────────────────────────────────────────
echo ACTIVE CODESPACES:
echo ─────────────────────────────────────────────────
gh cs list

echo.
echo ─────────────────────────────────────────────────
echo Refreshing in 30 seconds...
echo Press Ctrl+C to stop
timeout /t 30 /nobreak >nul
goto loop