# Desktop dictation

Settings → Dictation offers an optional 670,479,942-byte download of NVIDIA
Parakeet TDT 0.6B v3. Its consent text names Hugging Face and the download
size before you enable it. Nothing opens the microphone, downloads the
model, or contacts a dictation service at startup or while dictation is off.
Turning it off retains the model. Remove deletes its cache and disables it.
This is separate from Noches' existing networked GPT-Live voice call.

Hold the microphone, Enter/Space on its focused button, or Cmd/Ctrl+Alt+R.
Release to transcribe into the selected range of this pane's draft.
A tap shorter than 300 ms explains hold to talk without transcribing.
Rebind Hold to dictate in Settings → Shortcuts. Cmd/Ctrl+D remains Split pane
right. An accessibility Click starts capture, and another finishes it.
Esc or Cancel discards unfinished recognition without erasing the draft.

Transcription runs on this desktop, with CPU ONNX Runtime 1.28.0 through
parakeet-rs 0.3.8 and ort 2.0.0-rc.13. No audio file is saved; audio is neither
logged, attached, synced, nor sent to the engine host. Hugging Face is used
only for the explicit model download, pinned to revision
`8f23f0c03c8761650bdb5b40aaf3e40d2c15f1ce` with per-file SHA-256 checks.
The conversion publisher does not identify its exact source-weight revision
or INT8 conversion tool version. See `crates/dictation/NOTICE.md`.

Recognition is final-only. Recording is limited to 60 seconds and finalization
to 30 seconds. Capture starts independently of model loading. Stop closes
capture and retains audio while loading finishes. Cancellation closes capture
asynchronously and rejects late results. Native inference cannot be forcibly
interrupted; the single worker rejects overlap until it returns. Cached weights
unload after 30 idle seconds; removal is refused while native work is busy.

Dictated text is editable and never sends itself. Explicit Send or Queue waits
for finalization once. Blank recognition, failure, and timeout preserve the
draft and do not send. Editing, moving the selection, IME, navigation,
queue-draft replacement, and focus leaving the composer invalidate the session.
Each pane owns its draft generation. No wire or document fields change.

CPAL 0.15.3 is shared with GPT-Live rather than importing CPAL 0.17.3, which
conflicts with Noches' ALSA native link. Device names identify inputs because
CPAL 0.15 lacks stable platform ids. Identically named microphones cannot be
distinguished; a disconnected selection falls back to the system default.
The settings page refreshes devices only while enabled and visible.
Rubato 0.16.2 converts supported device rates to 16 kHz with anti-aliasing.

macOS requests microphone permission on first use through AVFoundation.
Both bundle plists describe local dictation and GPT-Live; the existing
`voice.entitlements` already grants hardened-runtime audio input.
Windows desktop microphone privacy access must be enabled. Linux needs ALSA
headers to build and `libasound.so.2` to run, already required by GPT-Live.
OpenSSL headers are needed by the ONNX archive downloader at build time.
The pinned distribution supports macOS arm64, Linux x86_64/aarch64 and Windows
x86_64. Intel macOS needs a separately built matching ONNX Runtime.
Windows' ONNX archive also links DirectML/DX12 even with CPU execution.
The new runtime adds native code to the binary, not the 670 MB weights.

`cargo test -p zeron-voice --locked` runs without a microphone or model.
`cargo test -p zeron-ui --lib --locked` covers injected editor events.
The `verify` example accepts a model directory followed by mono PCM16 WAVs;
it refuses to download unless `--download` follows the directory.
Production never prints transcripts. For startup timing,
`RUST_LOG=info,zeron_ui::dictation=debug` logs `activation_to_listening_ms`
without speech content. Real capture, recognition quality, native platform
permission prompts, packaging, and visual QA require separate verification.
