# Mori Desktop Companion
## Local AI Desktop Assistant — Product & Technical Specification

**Document type:** Product Specification + Technical Architecture + MVP Roadmap  
**Status:** Proposed — Awaiting Phase 0 approval  
**Target platform:** Windows 11  
**Architecture:** Local-first, Offline-capable, AI-powered Desktop Companion  
**Project codename:** Mori  
**Last updated:** 2026-11-11

---

# 1. Product Vision

Mori is a local AI desktop companion designed to provide natural Vietnamese voice conversations, persistent user-approved memory, a consistent personality, and an animated 2D virtual character.

Mori is not simply a chatbot with a microphone. It is intended to be a persistent desktop assistant that can:

- Listen to the user through a microphone.
- Understand and respond naturally in Vietnamese.
- Speak using a natural Vietnamese voice.
- Maintain a consistent companion personality.
- Remember information explicitly approved by the user.
- Display an animated 2D avatar on the Windows desktop.
- Provide useful assistance for programming, learning, writing, troubleshooting, and daily tasks.
- Operate without cloud AI services or external API dependencies.

## 1.1. Confirmed Product Direction

| Attribute | Decision |
|---|---|
| Avatar | 2D animated character |
| Avatar technology | Live2D |
| Voice | Natural Vietnamese voice |
| Interaction | Voice and text |
| AI execution | 100% local |
| Personality | Friendly companion |
| Memory | Local, user-controlled |
| Platform | Windows 11 |
| Desktop behavior | Floating, always-on-top companion |
| Cloud AI | Not allowed |
| Cloud fallback | Not allowed |

## 1.2. Product Principles

1. Local-first: all AI inference and personal data processing must happen locally.
2. Privacy-first: no silent recording, cloud transmission, telemetry, or hidden data collection.
3. Modular architecture: LLM, STT, TTS, avatar, and memory must be independently replaceable.
4. CPU fallback: the application must remain usable without GPU acceleration.
5. User control: memory, microphone, model downloads, and system integrations require explicit controls.
6. Reliability: AI functionality must not depend on avatar rendering.
7. Transparency: the application must clearly show when it is listening, thinking, speaking, or idle.

---

# 2. Target Hardware

The initial target machine is the user's ASUS Zenbook 14 UX3405CA.

## 2.1. Verified Hardware Configuration

| Component | Specification |
|---|---|
| Laptop | ASUS Zenbook 14 UX3405CA |
| CPU | Intel Core Ultra 9 285H |
| Logical processors | 16 |
| RAM | 32 GB |
| Usable RAM | Approximately 31.36 GB |
| GPU | Intel Arc 140T integrated GPU |
| GPU memory | Shared system memory; 128 MB dedicated memory reported |
| NPU | Intel AI Boost |
| NPU driver | 32.0.100.4023 |
| GPU driver | 32.0.101.8860 |
| DirectX | DirectX 12 |
| WDDM | 3.2 |
| Operating system | Windows 11 |
| Windows build | 26300.9711 |
| Storage | Approximately 1 TB Micron NVMe |
| Data root | E:\Memora Data\ |
| Pagefile | Fixed 2,048 MB |

## 2.2. Hardware Strategy

The application must not assume that the integrated GPU or NPU will accelerate every AI model.

Recommended initial strategy:

- CPU execution must be supported for every essential AI component.
- Intel Arc acceleration should be tested through compatible runtimes.
- NPU acceleration is optional and must not be a dependency.
- Model loading must be lazy.
- Avoid running multiple large AI models simultaneously.
- Resource management must monitor RAM, CPU, GPU, and worker processes.

The actual performance of each model must be measured on the target laptop.

TOPS figures and benchmark results from other computers must not be treated as proof of real-world performance.

---

# 3. System Architecture

## 3.1. High-Level Architecture

```text
                 Windows Desktop
                       |
               Tauri 2 + React
                       |
                Rust Application Core
                       |
          +------------+-------------+
          |                          |
     Voice Pipeline             Companion Engine
          |                          |
     +----+----+                +----+----+
     |         |                |         |
    STT       TTS           Personality  Memory
     |         |                |         |
     +----+----+                +----+----+
          |                          |
          +------------+-------------+
                       |
                 Local AI Runtime
                       |
             +---------+---------+
             |                   |
          Local LLM          SQLite Storage
             |                   |
             +---------+---------+
                       |
                 Windows Hardware
```

## 3.2. Architecture Rules

- Frontend must not directly load AI models.
- Frontend must not execute FFmpeg, Python, or model binaries.
- Rust owns application lifecycle and process management.
- AI workers communicate through versioned interfaces.
- Model runtimes must be independently replaceable.
- All local services must bind to loopback only.
- No cloud fallback is allowed.
- No AI worker may execute arbitrary operating-system commands.
- Worker crashes must not crash the entire desktop application.

## 3.3. Main Modules

| Module | Responsibility |
|---|---|
| Desktop UI | Chat, avatar, settings, floating window |
| Application Core | Lifecycle, events, process management |
| Audio Engine | Microphone capture and playback |
| Voice Pipeline | VAD, STT, LLM, TTS orchestration |
| LLM Runtime | Local language model inference |
| Personality Engine | Companion personality and response style |
| Memory Engine | User-approved persistent memory |
| Conversation Manager | Conversation history and context |
| Model Manager | Model installation and validation |
| Avatar Engine | Animation, expressions, lip-sync |
| Database | Conversations, messages, memories, settings |
| Security Layer | Permissions, local-only enforcement |
| Logging | Local diagnostics and error reporting |

---

# 4. Technology Stack

## 4.1. Proposed Stack

| Layer | Technology |
|---|---|
| Desktop framework | Tauri 2 |
| Frontend | React |
| Language | TypeScript |
| Styling | Tailwind CSS |
| Backend | Rust |
| Avatar | Live2D Cubism SDK for Web |
| Rendering | WebGL |
| LLM runtime | llama.cpp |
| LLM acceleration | SYCL for Intel GPU, CPU fallback |
| STT | whisper.cpp |
| STT alternative | PhoWhisper |
| TTS primary candidate | KorvaTTS |
| TTS alternatives | VieNeu-TTS, ZeroTTS |
| VAD | Silero VAD or WebRTC VAD |
| Audio capture | CPAL or another suitable Rust audio subsystem |
| Database | SQLite |
| Full-text search | SQLite FTS5 |
| Logging | tracing |
| Worker protocol | JSON Lines over stdio |
| IPC | Typed Tauri commands and events |

Technology choices are proposals until validated during Phase 0.

---

# 5. LLM — Local AI Brain

## 5.1. Primary Candidate

**Qwen3-4B GGUF Q4_K_M**

Model reference:

https://huggingface.co/Qwen/Qwen3-4B

Reasons for selection:

- Multilingual capabilities.
- Vietnamese conversation support.
- Instruction following.
- Reasoning capabilities.
- Suitable initial model size for a 32 GB laptop.
- Quantized GGUF variants are available for llama.cpp.

## 5.2. Initial Configuration

| Parameter | Proposed Value |
|---|---|
| Model | Qwen3-4B |
| Format | GGUF |
| Quantization | Q4_K_M |
| Context | 8K tokens initially |
| Default mode | Non-thinking |
| Reasoning mode | Enabled selectively |
| Runtime | llama.cpp |
| GPU backend | SYCL, if validated |
| Fallback | CPU |

## 5.3. Runtime Strategy

llama.cpp is the preferred initial runtime because it provides:

- GGUF model support.
- Configurable inference.
- Local server capability.
- CPU inference.
- Intel GPU acceleration options through SYCL.
- Model and context configuration.

The exact llama.cpp build and Intel Arc compatibility must be validated on the target Windows environment.

Do not assume GPU offloading works until verified.

## 5.4. Alternative Model Evaluation

A newer Qwen model may be evaluated later.

Do not replace the selected model merely because a newer release exists.

Evaluate:

- Vietnamese response quality.
- First-token latency.
- Tokens per second.
- RAM usage.
- GPU compatibility.
- Context stability.
- License and redistribution conditions.

---

# 6. Speech-to-Text

## 6.1. Primary Candidate: whisper.cpp

Repository:

https://github.com/ggml-org/whisper.cpp

Initial model candidates:

- Whisper multilingual base.
- Whisper multilingual small.

The small model is preferred if its latency and resource usage remain acceptable.

## 6.2. Alternative: PhoWhisper

PhoWhisper is a Vietnamese speech-recognition model developed by VinAI.

It should be evaluated for:

- Vietnamese accents.
- Southern Vietnamese speech.
- Proper nouns.
- Programming terminology.
- Mixed Vietnamese and English speech.

Repository/model references must be checked before implementation.

PhoWhisper must not be assumed to be directly compatible with whisper.cpp. Conversion and runtime requirements must be evaluated.

## 6.3. Voice Input Workflow

```text
Microphone
    |
Audio Capture
    |
Voice Activity Detection
    |
Speech Segmentation
    |
Speech-to-Text
    |
Transcript Validation
    |
Conversation Manager
```

## 6.4. Initial Interaction Mode

MVP must start with push-to-talk.

Wake-word detection is deferred.

Requirements:

- Microphone permission.
- Device selection.
- Clear microphone status.
- Immediate stop button.
- No background recording by default.
- No permanent audio recording.
- Temporary audio buffers must be removed after processing.

---

# 7. Text-to-Speech

Natural Vietnamese speech is a major product requirement.

The TTS component should prioritize naturalness, pronunciation, pacing, and low latency.

## 7.1. Primary Candidate: KorvaTTS

Repository:

https://github.com/dogenthq/KorvaTTS

Reasons for evaluation:

- Vietnamese-focused TTS.
- On-device execution.
- ONNX Runtime.
- Potentially suitable for CPU inference.
- Support for Vietnamese-English mixed text.

## 7.2. Alternative Candidates

### VieNeu-TTS

Repository:

https://github.com/bonelag/VieNeuTTS

Evaluate for:

- Vietnamese voice naturalness.
- Preset voice selection.
- Expressiveness.
- CPU inference.
- Memory requirements.

### ZeroTTS

Repository:

https://github.com/zeroweight-ai/ZeroTTS

Evaluate as an additional local TTS candidate.

Do not rely on performance claims from external demonstrations. Benchmark on the target laptop.

## 7.3. TTS Evaluation Criteria

| Criterion | Measurement |
|---|---|
| Naturalness | Human evaluation |
| Vietnamese pronunciation | Pronunciation test set |
| Pacing | Human evaluation |
| English terminology | Mixed-language test |
| First audio latency | Seconds |
| Real-time factor | Processing time / audio duration |
| CPU usage | Percentage |
| RAM usage | MB/GB |
| Long text stability | Stress test |

## 7.4. Audio Streaming

The application should generate speech in chunks.

Recommended workflow:

1. LLM produces the first sentence.
2. TTS begins synthesizing the first sentence.
3. Audio playback starts.
4. Remaining sentences are synthesized progressively.
5. Avatar lip-sync follows the active audio.
6. User interruption cancels pending TTS chunks.

---

# 8. Voice Activity Detection

VAD detects when the user starts and stops speaking.

Candidates:

- Silero VAD.
- WebRTC VAD.

MVP requirements:

- Detect speech start.
- Detect speech end.
- Avoid excessively short segments.
- Handle brief pauses.
- Avoid triggering on normal background noise.
- Allow manual push-to-talk override.

Wake-word support is deferred until the basic voice pipeline is stable.

---

# 9. Avatar System

## 9.1. Technology

Live2D Cubism SDK for Web.

Official reference:

https://www.live2d.com/en/sdk/download/web/

Rendering through WebGL inside the Tauri WebView.

## 9.2. Avatar States

| State | Behavior |
|---|---|
| Idle | Blinking, breathing, subtle movement |
| Listening | Focused expression |
| Thinking | Looking aside, thinking animation |
| Speaking | Mouth movement |
| Happy | Smiling |
| Confused | Head tilt |
| Sleeping | Eyes closed, reduced animation |

## 9.3. Lip-Sync

MVP:

Volume-based lip-sync.

- Analyze audio amplitude.
- Map amplitude to mouth opening.
- Smooth transitions.
- Avoid unnatural rapid mouth movement.

Future:

Viseme-based lip-sync.

This may provide more accurate mouth shapes based on speech phonemes.

## 9.4. Desktop Behavior

- Transparent window.
- Always-on-top.
- Draggable.
- Resizable.
- Configurable opacity.
- Optional click-through.
- Adjustable position.
- Low-resource idle mode.

## 9.5. Display Modes

### Companion Mode

Only the character is visible.

### Chat Mode

Character and conversation panel are visible together.

### Focus Mode

Character is hidden while background services remain available.

## 9.6. Licensing

Live2D SDK, model assets, character artwork, and redistribution terms must be reviewed separately.

Do not assume a model downloaded from the Internet is permitted for commercial redistribution.

---

# 10. Personality Engine

## 10.1. Personality Name

Mori.

## 10.2. Personality Direction

Friendly, intelligent, thoughtful, slightly humorous, and emotionally aware in communication style.

Mori should feel like a companion rather than a generic customer-support chatbot.

## 10.3. Behavioral Principles

- Friendly and natural Vietnamese.
- Can use light humor when appropriate.
- Serious when handling important work.
- Does not excessively flatter the user.
- Does not pretend to know unknown information.
- Can politely challenge incorrect assumptions.
- Can ask clarifying questions.
- Does not continuously interrupt or initiate conversation without permission.
- Maintains a consistent personality.

## 10.4. Example Responses

User: "Mori ơi."

Mori: "Mình đây, bạn cần mình giúp gì?"

User: "Code này bị lỗi."

Mori: Focuses on identifying the root cause and proposing a practical fix.

User: "Hôm nay hơi mệt."

Mori: Responds gently without pressuring the user to continue working.

## 10.5. Configuration Files

```text
config/
├── personality.yaml
├── conversation_policy.yaml
├── voice_style.yaml
└── assistant_settings.yaml
```

Personality and safety rules must be separated.

---

# 11. Memory Engine

Memory must remain local and user-controlled.

## 11.1. Memory Layers

### Short-term Memory

- Current conversation.
- Recent messages.
- Current task context.
- Temporary facts.

### Long-term Memory

- User-approved preferences.
- Projects.
- Technical decisions.
- Repeatedly useful information.
- User-approved personal context.

### Knowledge Memory

- User-selected documents.
- Notes.
- Project documentation.
- Imported knowledge.

## 11.2. Storage

SQLite is sufficient for MVP.

SQLite FTS5 should be used for full-text search.

Vector search is deferred until semantic retrieval is demonstrably useful.

## 11.3. Initial Database Schema

```sql
CREATE TABLE conversations (
    id TEXT PRIMARY KEY,
    title TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE messages (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (conversation_id)
        REFERENCES conversations(id)
);

CREATE TABLE memories (
    id TEXT PRIMARY KEY,
    category TEXT NOT NULL,
    content TEXT NOT NULL,
    source TEXT,
    confidence REAL,
    user_approved INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

## 11.4. Memory Rules

- AI must not automatically save every personal conversation.
- AI may propose memories.
- User can approve, edit, or reject a memory.
- User can inspect stored memories.
- User can delete individual memories.
- User can delete all local memory.
- Deletion must be verified.
- Database backups must be supported.

---

# 12. Conversation Manager

Conversation Manager orchestrates the complete voice interaction.

## 12.1. Workflow

```text
User speaks
    |
VAD detects speech
    |
STT generates transcript
    |
Conversation Manager builds context
    |
LLM generates response
    |
Response split into sentences
    |
TTS synthesizes chunks
    |
Audio playback
    |
Avatar lip-sync
```

## 12.2. Responsibilities

- Manage conversation history.
- Build model context.
- Summarize long conversations.
- Prevent repeated audio playback.
- Support interruption.
- Cancel active generation.
- Cancel pending TTS.
- Manage conversation state.
- Coordinate avatar state.
- Keep memory retrieval separate from the main prompt.

---

# 13. Worker Architecture and IPC

Rust manages native processes.

Potential workers:

- llama-server.exe.
- whisper-cli.exe or an STT worker.
- Python TTS worker.
- VAD worker if required.

Python should only be used where model ecosystem compatibility requires it.

## 13.1. Worker Protocol

JSON Lines over stdin/stdout is acceptable for initial worker communication.

Example:

```json
{
  "protocol_version": 1,
  "request_id": "req-001",
  "type": "synthesize",
  "payload": {
    "text": "Chào bạn, hôm nay mình có thể giúp gì?"
  }
}
```

## 13.2. Requirements

Every worker request must support:

- Request ID.
- Protocol version.
- Timeout.
- Cancellation.
- Health checks.
- Output validation.
- Payload size limits.
- Graceful shutdown.

Worker logs must use stderr.

stdout must contain protocol messages only.

Worker crashes must not terminate the desktop UI.

---

# 14. Privacy and Security

Strict local execution is mandatory.

## 14.1. Non-negotiable Requirements

| Component | Policy |
|---|---|
| LLM | Local only |
| STT | Local only |
| TTS | Local only |
| Memory | Local SQLite |
| Voice recordings | Not retained by default |
| Logs | No sensitive content by default |
| Telemetry | Disabled |
| Cloud API | Not allowed |
| Cloud fallback | Not allowed |
| Model downloads | Explicit approval |
| Web access | Disabled in MVP |
| Crash reporting | Local only |

## 14.2. Security Requirements

- Local services bind to 127.0.0.1.
- No inbound LAN access.
- No hidden cloud requests.
- No silent recording.
- Microphone status must be visible.
- Stop must immediately stop audio capture.
- Model files must be checksum-verified.
- Model licenses must be reviewed.
- Memory deletion must be reliable.
- AI cannot execute arbitrary shell commands.
- Skills require explicit permission.
- No unrestricted filesystem access.

## 14.3. Offline Requirements

Initial installation and model acquisition may require Internet access.

After dependencies and models are installed, core conversation functionality must work offline.

The application should provide an offline readiness check.

---

# 15. Resource Management

The following values are initial planning estimates, not measured benchmarks.

| Component | Estimated RAM Budget |
|---|---:|
| Windows and background services | 6–9 GB |
| LLM | 3–6 GB |
| STT | 0.5–2 GB |
| TTS | 0.5–2 GB |
| UI and avatar | 0.3–1 GB |
| Database and buffers | 0.2–0.5 GB |

## 15.1. Scheduling Policy

- Prioritize LLM during active conversations.
- Run STT only when required.
- Run TTS only when generating speech.
- Avoid unnecessary simultaneous GPU workloads.
- Use lazy model loading.
- Release unused model resources where practical.
- Target 30 FPS for active avatar animation.
- Reduce animation activity when idle.
- Provide a Pause AI function.

## 15.2. Initial Hardware Assignment

| Component | Preferred Execution |
|---|---|
| LLM | CPU initially, SYCL evaluation |
| STT | CPU |
| TTS | CPU / ONNX |
| Avatar | WebGL |
| NPU | Not required for MVP |

---

# 16. Data Storage

Suggested data root:

```text
E:\Mori Data\
├── models\
│   ├── llm\
│   ├── stt\
│   ├── tts\
│   └── manifests\
├── database\
├── memory\
├── avatar\
├── audio-cache\
├── logs\
├── backups\
└── settings\
```

Model weights must not be committed to Git.

The models directory should contain manifests, installation metadata, checksums, and model state.

Model Manager should support:

- Model listing.
- Model installation.
- Model removal.
- Version tracking.
- Checksum validation.
- Disk usage display.
- Runtime compatibility checks.
- Default model selection.
- Backend status display.
- Offline readiness checks.

---

# 17. Project Source Structure

```text
Mori/
├── CLAUDE.md
├── README.md
├── docs/
│   ├── PRODUCT_SPEC.md
│   ├── TECHNICAL_ARCHITECTURE.md
│   ├── MODEL_SELECTION.md
│   ├── SECURITY_AND_PRIVACY.md
│   ├── MVP_ROADMAP.md
│   ├── TEST_PLAN.md
│   ├── HARDWARE_VALIDATION.md
│   └── ADR/
│
├── src/
│   ├── app/
│   ├── components/
│   │   ├── avatar/
│   │   ├── chat/
│   │   ├── voice/
│   │   └── settings/
│   ├── features/
│   │   ├── conversation/
│   │   ├── memory/
│   │   ├── personality/
│   │   ├── audio/
│   │   └── model-manager/
│   ├── services/
│   ├── hooks/
│   └── types/
│
├── src-tauri/
│   ├── src/
│   │   ├── main.rs
│   │   ├── commands/
│   │   ├── audio/
│   │   ├── conversation/
│   │   ├── database/
│   │   ├── model_runtime/
│   │   ├── process/
│   │   ├── permissions/
│   │   ├── settings/
│   │   └── logging/
│   └── capabilities/
│
├── workers/
│   ├── stt/
│   ├── tts/
│   └── vad/
│
├── models/
│   ├── manifests/
│   └── README.md
│
├── assets/
│   ├── avatar/
│   ├── audio/
│   └── themes/
│
├── tests/
│   ├── integration/
│   ├── performance/
│   ├── audio/
│   └── fixtures/
│
└── scripts/
```

---

# 18. MVP Roadmap

Implementation must proceed phase by phase.

## Phase 0 — Discovery and Feasibility

Objectives:

- Inspect Windows environment.
- Validate Intel driver and GPU availability.
- Validate microphone and audio devices.
- Evaluate Qwen3-4B GGUF.
- Test llama.cpp CPU.
- Test llama.cpp SYCL.
- Compare Whisper small and PhoWhisper.
- Compare KorvaTTS, VieNeu-TTS, and ZeroTTS.
- Review Live2D SDK and model licensing.
- Measure RAM, CPU, GPU, latency, and quality.

Deliverables:

- HARDWARE_REPORT.md
- MODEL_EVALUATION.md
- TECHNICAL_DECISIONS.md
- ADR documents.

Gate:

At least one viable offline LLM, STT, and TTS configuration must be demonstrated.

No model download or installation without approval.

## Phase 1 — Local AI Core

- Tauri shell.
- Rust process manager.
- llama.cpp integration.
- Chat UI.
- Streaming tokens.
- Stop generation.
- CPU fallback.

Deliverable: local text-based conversation.

Gate: core chat works offline after model installation.

## Phase 2 — Voice Input

- Microphone capture.
- VAD.
- STT.
- Transcript preview.
- Push-to-talk.
- Device selection.

Deliverable: Vietnamese speech becomes usable conversation input.

Gate: recognition works across multiple speech styles and noise conditions.

## Phase 3 — Voice Output

- TTS worker.
- Audio streaming.
- Playback.
- Interrupt speech.
- Voice configuration.

Deliverable: Mori responds with Vietnamese speech.

Gate: natural, stable speech without repeated or missing segments.

## Phase 4 — Personality and Memory

- Personality prompt.
- Conversation Manager.
- SQLite memory.
- Conversation search.
- Memory approval.
- Memory editing and deletion.

Deliverable: consistent personality and controlled memory.

Gate: multi-turn conversations and memory recovery pass tests.

## Phase 5 — Avatar Companion

- Live2D integration.
- Transparent floating window.
- Avatar states.
- Lip-sync.
- Always-on-top.
- Hotkey support.

Deliverable: animated desktop companion.

Gate: stable animation without materially harming AI performance.

## Phase 6 — MVP Stabilization

- Full benchmark.
- Offline verification.
- Crash recovery.
- Backup and restore.
- Audio recovery.
- Security audit.
- Regression testing.

Deliverable: stable MVP for daily use.

Gate: all critical acceptance criteria pass.

---

# 19. Testing Strategy

## 19.1. LLM Tests

| ID | Test | Expected Result |
|---|---|---|
| AI-01 | Vietnamese conversation | No language corruption |
| AI-02 | Multi-turn conversation | Context retained |
| AI-03 | Programming question | Useful technical explanation |
| AI-04 | Unknown information | No fabricated certainty |
| AI-05 | Personality consistency | Stable companion style |
| AI-06 | Prompt injection | No permission escalation |
| AI-07 | Stop generation | Generation stops promptly |
| AI-08 | Long context | Important information retained |

## 19.2. Vietnamese STT Tests

Build a 100-sentence test set:

- 20 everyday conversation sentences.
- 20 Southern Vietnamese speech samples.
- 20 sentences containing English and technical terminology.
- 20 long sentences with numbers and punctuation.
- 20 noisy or fast speech samples.

Measure:

- Word Error Rate.
- Character Error Rate.
- Proper noun errors.
- Technical terminology errors.
- Vietnamese tone mark errors.
- Human-rated transcript quality.

## 19.3. TTS Tests

Evaluate:

- Naturalness.
- Pronunciation.
- Pacing.
- Pauses.
- Mixed Vietnamese-English reading.
- First audio latency.
- Real-time factor.
- CPU usage.
- RAM usage.
- Long text stability.

## 19.4. Performance Targets

These are initial design targets, not verified measurements.

| Metric | Initial Target |
|---|---:|
| Short utterance STT P95 | Under 2 seconds |
| LLM time-to-first-token P95 | Under 2 seconds |
| TTS first audio P95 | Under 1.5 seconds |
| Total initial response latency P95 | Under 4 seconds |
| Active avatar FPS | Target 30 FPS |
| Microphone interruption | Under 300 ms |

Measure separately:

- Cold start.
- Warm start.
- Time to first token.
- Tokens per second.
- TTS real-time factor.
- End-of-speech to first audible response.

## 19.5. Resource Tests

| Test | Expected Result |
|---|---|
| 30-minute idle | No continuous memory growth |
| 30-minute conversation | No worker hang |
| 2-hour conversation | Stable memory usage |
| GPU unavailable | CPU fallback works |
| Continuous TTS | No audio pipeline blockage |
| Audio device change | Capture/playback recovers |
| Sleep/resume | Workers recover safely |

## 19.6. Privacy Tests

- Disconnect Internet and verify core conversation.
- Inspect outbound network traffic.
- Verify microphone stops after Stop.
- Verify temporary audio is deleted.
- Verify database does not contain credentials.
- Verify local server is not accessible from another LAN device.
- Verify memory deletion.
- Verify no automatic model downloads.
- Verify no automatic updates while offline.

## 19.7. Human Evaluation

Score each category from 1 to 5.

| Criterion | Score |
|---|---:|
| Vietnamese understanding | 1–5 |
| Voice naturalness | 1–5 |
| Speech pacing | 1–5 |
| Personality consistency | 1–5 |
| Memory usefulness | 1–5 |
| Conversation experience | 1–5 |
| Avatar expressiveness | 1–5 |

Proposed acceptance threshold: average score of at least 4/5 for core experience criteria, with no critical privacy or data-loss defects.

---

# 20. Risks and Mitigation

| Risk | Severity | Mitigation |
|---|---|---|
| SYCL instability | High | CPU fallback |
| TTS not natural enough | High | Compare three candidates |
| STT technical terminology errors | High | PhoWhisper evaluation and glossary |
| LLM latency | High | 4B Q4, context tuning, streaming |
| Multiple models consume RAM | High | Lazy loading and resource manager |
| Avatar performance overhead | Medium | Lower idle FPS |
| Live2D licensing | Medium | Review distribution rights |
| Unwanted memory storage | High | Approval and deletion |
| Worker crashes | Medium | Health checks and restart |
| Audio echo | Medium | Echo cancellation and routing |

---

# 21. Deferred Features

Not part of MVP:

- Always-on wake-word detection.
- Face recognition.
- Screen understanding.
- Automatic Windows control.
- Voice cloning.
- Third-party plugins.
- Cloud services.
- Internet search.
- Unrestricted shell execution.
- Automatic file scanning.

These features require separate design, permissions, and approval.

---

# 22. Claude Code Development Rules

Claude Code is a development assistant, not the runtime AI of Mori.

## 22.1. Approval Rules

- Phase 0 is read-only environment inspection and planning.
- Do not download models without explicit user