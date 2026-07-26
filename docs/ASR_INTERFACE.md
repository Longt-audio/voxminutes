# VoxMinutes MVP ASR 接口设计

## 1. 设计目标

VoxMinutes MVP 需要支持两类本地 ASR 引擎，并为未来云端/第三方 ASR 预留接口：

- **实时流式转录**（麦克风 / 系统音频）
- **离线文件转写**（导入音频文件）

本地引擎：

| 引擎 | 类型 | 适用场景 | 备注 |
|------|------|----------|------|
| `xasr` | 纯流式 | 实时转录，低延迟 | 基于 sherpa-onnx X-ASR |
| `sensevoice` | VAD 伪流式 | 实时转录 + 离线文件 | 基于 sherpa-onnx SenseVoice，用 VAD 切分 |
| `remote` | 预留接口 | 未来云端/第三方 ASR | 仅定义接口，MVP 不实现 |

---

## 2. ASR Provider Trait（Rust）

```rust
use async_trait::async_trait;
use std::path::Path;

/// A chunk of audio data fed to the ASR engine.
#[derive(Clone)]
pub struct AudioChunk {
    pub data: Vec<i16>,
    pub sample_rate: u32,
    pub channels: u16,
    pub timestamp_ms: u64,
}

/// A single ASR result segment.
#[derive(Clone, Debug)]
pub struct AsrSegment {
    pub text: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub is_final: bool,
    pub speaker: Option<String>,
    pub language: Option<String>,
}

/// Configuration for creating an ASR provider.
#[derive(Clone, Debug)]
pub struct AsrConfig {
    pub engine: AsrEngine,
    pub model_path: std::path::PathBuf,
    pub language: String,
    pub sample_rate: u32,
    // Provider-specific options, serialized as JSON.
    pub extra: serde_json::Value,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AsrEngine {
    XAsr,
    SenseVoice,
    Remote,
}

#[async_trait]
pub trait AsrProvider: Send + Sync {
    /// Initialize the engine (load models, etc.).
    async fn init(&mut self) -> anyhow::Result<()>;

    /// Feed a chunk of audio for streaming ASR.
    /// Returns partial/final segments as they become available.
    async fn feed_audio(&mut self, chunk: AudioChunk) -> anyhow::Result<Vec<AsrSegment>>;

    /// Signal end of stream and flush any remaining results.
    async fn finalize(&mut self) -> anyhow::Result<Vec<AsrSegment>>;

    /// Transcribe an entire audio file (offline).
    async fn transcribe_file(&mut self, path: &Path) -> anyhow::Result<Vec<AsrSegment>>;

    /// Reset internal state for a new session.
    async fn reset(&mut self) -> anyhow::Result<()>;
}

/// Factory function to create a provider from config.
pub fn create_asr_provider(config: AsrConfig) -> anyhow::Result<Box<dyn AsrProvider>> {
    match config.engine {
        AsrEngine::XAsr => Ok(Box::new(XAsrProvider::new(config)?)),
        AsrEngine::SenseVoice => Ok(Box::new(SenseVoiceProvider::new(config)?)),
        AsrEngine::Remote => Ok(Box::new(RemoteAsrProvider::new(config)?)),
    }
}
```

---

## 3. 本地引擎实现策略

### 3.1 X-ASR 纯流式

- 直接接收固定长度的音频流（建议 320ms ~ 960ms）。
- 每收到一个 chunk，立即送入 sherpa-onnx X-ASR 模型。
- 返回 `is_final=false` 的 partial 结果，句子结束时返回 `is_final=true`。
- 不需要 VAD。

实现文件：`frontend/src-tauri/src/audio/transcription/x_asr_provider.rs`（已存在，保留改造）

### 3.2 SenseVoice VAD 伪流式

- 用 Silero VAD 检测语音段落。
- 当 VAD 检测到一段完整语音后，送入 sherpa-onnx SenseVoice 模型转写。
- 返回 `is_final=true` 的完整段落。
- 文件离线转写也走此流程：先 VAD 切分，再批量转写。

实现文件：`frontend/src-tauri/src/audio/transcription/sherpa_onnx_provider.rs`（已存在，保留改造）

### 3.3 统一调用入口

```rust
// audio/transcription/engine.rs
pub async fn create_streaming_asr(
    engine: AsrEngine,
    model_dir: &Path,
    language: &str,
) -> anyhow::Result<Box<dyn AsrProvider>> {
    let config = AsrConfig {
        engine,
        model_path: model_dir.to_path_buf(),
        language: language.to_string(),
        sample_rate: 16000,
        extra: serde_json::json!({}),
    };
    create_asr_provider(config)
}
```

---

## 4. 远程 ASR API 预留接口

MVP 阶段不实现远程 ASR，但接口保留，方便后续付费功能接入。

### 4.1 配置结构

```rust
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RemoteAsrConfig {
    pub endpoint: String,
    pub api_key: String,
    pub model: String,
    pub language: String,
}
```

### 4.2 预留方法（RemoteAsrProvider）

```rust
pub struct RemoteAsrProvider {
    config: RemoteAsrConfig,
}

#[async_trait]
impl AsrProvider for RemoteAsrProvider {
    async fn init(&mut self) -> anyhow::Result<()> {
        // MVP: only validate config, do not connect.
        Ok(())
    }

    async fn feed_audio(&mut self, _chunk: AudioChunk) -> anyhow::Result<Vec<AsrSegment>> {
        Err(anyhow::anyhow!("Remote ASR is not available in the open-source MVP."))
    }

    async fn finalize(&mut self) -> anyhow::Result<Vec<AsrSegment>> {
        Ok(vec![])
    }

    async fn transcribe_file(&mut self, _path: &Path) -> anyhow::Result<Vec<AsrSegment>> {
        Err(anyhow::anyhow!("Remote ASR is not available in the open-source MVP."))
    }

    async fn reset(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}
```

---

## 5. 数据流

### 5.1 实时转录

```
Audio Capture (CPAL)
        ↓
Audio Processing (resample, mono, 16kHz)
        ↓
AsrProvider::feed_audio(chunk)
        ↓
X-ASR or SenseVoice+VAD
        ↓
AsrSegment
        ↓
Save to SQLite (transcript_segments)
        ↓
Emit event to frontend (transcript-update)
```

### 5.2 离线文件转写

```
User selects audio file
        ↓
Decode to WAV (decoder.rs)
        ↓
Create AsrProvider (SenseVoice)
        ↓
AsrProvider::transcribe_file(path)
        ↓
VAD split + batch transcribe
        ↓
AsrSegment list
        ↓
Save to SQLite (recordings + transcript_segments)
        ↓
Show in history
```

---

## 6. 设置项

SQLite `settings` 表中存储以下键值对：

| key | value | 说明 |
|-----|-------|------|
| `asr.engine` | `xasr` / `sensevoice` | 默认 ASR 引擎 |
| `asr.model_dir` | 路径 | 模型目录 |
| `asr.language` | `zh` / `en` / `auto` | 默认语言 |
| `audio.input_device` | 设备 ID | 录音设备 |
| `audio.output_device` | 设备 ID | 系统音频回环设备 |
| `export.default_dir` | 路径 | 默认导出目录 |
| `remote_asr.endpoint` | URL | 远程 ASR 端点（预留） |
| `remote_asr.api_key` | string | 远程 ASR API key（预留） |
| `remote_asr.model` | string | 远程 ASR 模型名（预留） |

---

## 7. 模型目录结构

```
models/
├── xasr/
│   └── (X-ASR model files)
├── sensevoice/
│   └── (SenseVoice model files)
└── sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17/
    └── (legacy SenseVoice model)
```

下载脚本负责把模型放到上述位置。

---

## 8. 前端事件约定

前端通过 Tauri event 监听实时转录结果：

```typescript
// event name: transcript-update
interface TranscriptUpdateEvent {
  recording_id: string;
  segment: {
    text: string;
    start_ms: number;
    end_ms: number;
    is_final: boolean;
    speaker?: string;
  };
}
```

---

## 9. 后续改造清单

1. 把现有 `provider.rs` / `engine.rs` / `x_asr_provider.rs` / `sherpa_onnx_provider.rs` 改造为统一 trait。
2. 删除/简化 `remote_asr_provider.rs` 的实际网络逻辑，改为 stub。
3. 录音管理器 (`recording_manager.rs`) 使用 `AsrProvider` 替代当前硬编码引擎选择。
4. 文件导入 (`import.rs` / `retranscription.rs`) 使用 `AsrProvider::transcribe_file`。
5. 设置页面读取/保存 `settings` 表中的 ASR 配置。

---

## 10. 注意事项

- 本地优先：MVP 所有 ASR 计算在本地完成，音频不上传。
- 远程 ASR 仅作为预留接口，开源版不实现。
- 付费版可以在此 trait 基础上实现真正的 `RemoteAsrProvider` 或 `CloudAsrProvider`。
