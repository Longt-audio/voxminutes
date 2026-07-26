# Remote Qwen3-ASR Deployment Guide

## Architecture

```
[VoxMinutes App] --HTTP--> [vLLM Server (WSL/Linux)] --GPU--> [Qwen3-ASR Model]
   Windows                  localhost:8000                    Qwen/Qwen3-ASR-1.7B
```

The local Qwen3-ASR model has been replaced with a remote vLLM server. The app sends audio to the server using the OpenAI-compatible chat completions API.

## Prerequisites

### Server-side (WSL or Linux)
- NVIDIA GPU with CUDA support
- Python 3.10+
- vLLM: `pip install vllm`
- (Optional) Model pre-downloaded to avoid first-run delay

### Client-side (VoxMinutes App)
- Network access to the vLLM server endpoint
- Remote ASR endpoint configured in Settings page

## Starting the Server

### Option 1: Using the provided script

```bat
scripts\remote-asr\start-asr.bat
```

Custom model size or port:
```bat
scripts\remote-asr\start-asr.bat 1.7B 8000
```

### Option 2: Manual start in WSL

```bash
python3 -m vllm.entrypoints.openai.api_server \
  --model Qwen/Qwen3-ASR-1.7B \
  --max-model-len 16384 \
  --port 8000 \
  --trust-remote-code
```

## Configuring VoxMinutes

1. Open **Settings** in the VoxMinutes app
2. Find the **Remote ASR** section
3. Enter the server URL (e.g., `http://localhost:8000`)
4. Click **Health Check** to verify connectivity
5. Click **Save** to persist the configuration
6. Select the `qwen3-asr-remote` model from the model list

## Health Check

```bat
scripts\remote-asr\check-health.bat
```

Or manually:
```bash
curl http://localhost:8000/v1/models
```

## API Protocol

The remote ASR uses the OpenAI-compatible chat completions endpoint:

```
POST /v1/chat/completions
```

Request format:
```json
{
  "model": "Qwen/Qwen3-ASR-1.7B",
  "messages": [
    {
      "role": "user",
      "content": [
        {
          "type": "input_audio",
          "input_audio": {
            "data": "<base64-encoded-wav>",
            "format": "wav"
          }
        },
        {
          "type": "text",
          "text": "Please transcribe the following audio."
        }
      ]
    }
  ],
  "temperature": 0.0,
  "top_p": 1.0
}
```

Response parsing extracts text from `<asr_text>...</asr_text>` tags and language info from `<language .../>` tags.

## Audio Chunking

Audio segments longer than 25 seconds are automatically split into overlapping chunks:
- Chunk duration: 25 seconds
- Overlap: 1 second between chunks
- Chunks are transcribed independently and concatenated

This ensures compatibility with the vLLM `--max-model-len 16384` limit while preserving transcription quality at chunk boundaries.

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Server not responding | Check if vLLM is running: `curl localhost:8000/v1/models` |
| CUDA out of memory | Use smaller model or reduce `--max-model-len` |
| Slow first request | Model downloads on first run; pre-download with `huggingface-cli download` |
| Garbled transcription | Check audio format; ensure 16kHz mono WAV |
| Connection refused in WSL | Ensure WSL2 networking is working; try `wsl hostname -I` to get WSL IP |
| Port conflict | Change port in both `start-asr.bat` and VoxMinutes Settings |

## Local Model Removal

The local Qwen3-ASR model directory has been removed:
- `models/sherpa-onnx-qwen3-asr-0.6B-int8-2026-03-25/` — deleted

Local SenseVoice model remains unchanged at `models/sherpa-onnx-sense-voice-zh/`.
