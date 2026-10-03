# Parakeet v3 attribution and runtime

NVIDIA Parakeet TDT 0.6B v3 weights © NVIDIA, licensed under Creative Commons Attribution 4.0 International: https://creativecommons.org/licenses/by/4.0/

Original model: https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3
Source model card inspected at revision `541d1f99c6b0c3cd0b11a95167540bb8edefd82b`.

ONNX conversion by Ivan Stupakov: https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/tree/8f23f0c03c8761650bdb5b40aaf3e40d2c15f1ce
The conversion card identifies NVIDIA v3 as its base and documents NeMo ASRModel export and vocabulary extraction. Its exact original-weight revision and INT8 conversion tool version are not published. Noches pins the conversion's immutable revision and SHA-256 of every consumed artifact in model.json; it does not claim independent numerical equivalence to NVIDIA's FP32 weights. The change is an INT8 ONNX conversion; no fine-tuned model is used.

Runtime: parakeet-rs 0.3.8 (MIT OR Apache-2.0), ONNX Runtime 1.28.0 via ort/ort-sys 2.0.0-rc.13, CPU execution. Cargo.lock pins registry checksums and the transitive graph. ONNX Runtime is MIT licensed. Capture: cpal 0.15.3 (Apache-2.0), shared with Noches' GPT-Live backend to avoid conflicting ALSA links. Sample-rate conversion: rubato 0.16.2 (MIT), using its anti-aliasing FFT resampler to convert device-rate mono audio to 16 kHz. Runtime libraries are linked into the application by ort-sys; model weights are an optional download, never bundled.

TDT v3 is an offline model. Noches records at most 60 seconds and decodes once on Stop. There are no live partial hypotheses in the production adapter. The editor's partial-result seam exists for deterministic lifecycle tests and future adapter work; it does not imply native streaming support.

Supported languages: Bulgarian, Croatian, Czech, Danish, Dutch, English, Estonian, Finnish, French, German, Greek, Hungarian, Italian, Latvian, Lithuanian, Maltese, Polish, Portuguese, Romanian, Russian, Slovak, Slovenian, Spanish, Swedish, Ukrainian.

Platform baseline: native CPU ONNX and CPAL on macOS, Windows, and Linux. Linux build hosts need ALSA development headers in addition to existing desktop dependencies. macOS requires a packaged app with NSMicrophoneUsageDescription and the audio-input entitlement under hardened runtime. No Apple Speech entitlement, cloud provider, Python installation, or engine-host audio transport is used. Native macOS/Windows packaging and physical microphone capture remain unverified for this Noches port.

## ONNX Runtime 1.28.0

MIT License

Copyright (c) Microsoft Corporation

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

## Oniguruma

Copyright (c) 2002-2021 K.Kosako <kkosako0@gmail.com>
All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice,
   this list of conditions and the following disclaimer.
2. Redistributions in binary form must reproduce the above copyright notice,
   this list of conditions and the following disclaimer in the documentation
   and/or other materials provided with the distribution.

THIS SOFTWARE IS PROVIDED BY THE AUTHOR AND CONTRIBUTORS "AS IS" AND
ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
ARE DISCLAIMED. IN NO EVENT SHALL THE AUTHOR OR CONTRIBUTORS BE LIABLE
FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
SUCH DAMAGE.
