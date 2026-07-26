@echo off
REM ============================================================
REM Start Qwen3-ASR vLLM Server via WSL
REM
REM Prerequisites:
REM   - WSL2 installed with a Linux distribution
REM   - Python 3.10+ with vllm installed in WSL
REM   - Qwen3-ASR model downloaded in WSL
REM   - Port 8000 available
REM
REM Usage:
REM   start-asr.bat                (default: Qwen/Qwen3-ASR-1.7B, port 8000)
REM   start-asr.bat 1.7B           (explicit model size)
REM   start-asr.bat 1.7B 8000      (explicit model size and port)
REM ============================================================

setlocal enabledelayedexpansion

set "MODEL_SIZE=%~1"
set "PORT=%~2"

if "%MODEL_SIZE%"=="" set "MODEL_SIZE=1.7B"
if "%PORT%"=="" set "PORT=8000"

set "MODEL_NAME=Qwen/Qwen3-ASR-%MODEL_SIZE%"
set "MAX_MODEL_LEN=16384"

echo ============================================================
echo  Qwen3-ASR Remote Server Launcher
echo ============================================================
echo  Model:      %MODEL_NAME%
echo  Port:       %PORT%
echo  Max Length:  %MAX_MODEL_LEN%
echo ============================================================
echo.

echo Starting vLLM server in WSL...
echo.

wsl bash -c "python3 -m vllm.entrypoints.openai.api_server --model %MODEL_NAME% --max-model-len %MAX_MODEL_LEN% --port %PORT% --trust-remote-code 2>&1"

if %ERRORLEVEL% neq 0 (
    echo.
    echo [ERROR] vLLM server exited with error code %ERRORLEVEL%
    echo.
    echo Troubleshooting:
    echo   1. Ensure vllm is installed:  pip install vllm
    echo   2. Ensure the model is available (will auto-download on first run)
    echo   3. Ensure port %PORT% is not in use
    echo   4. Check GPU driver and CUDA compatibility in WSL
    echo.
)

pause
