@echo off
REM ============================================================
REM Health check for Qwen3-ASR remote server
REM ============================================================

set "PORT=%~1"
if "%PORT%"=="" set "PORT=8000"

echo Checking Qwen3-ASR server on port %PORT%...

curl -s http://localhost:%PORT%/v1/models | python -m json.tool 2>nul

if %ERRORLEVEL% neq 0 (
    echo [FAIL] Server not responding on http://localhost:%PORT%
    echo.
    echo Make sure the vLLM server is running. Use start-asr.bat to start it.
) else (
    echo.
    echo [OK] Server is responding.
)

echo.
pause
