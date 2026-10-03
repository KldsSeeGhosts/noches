# Third-party notices

Zeron bundles the following syntax-highlighting components. Unless noted otherwise, their parsers and queries are consumed from the pinned Rust crates listed in `Cargo.lock`. The Kotlin highlight query is maintained as Zeron source code and is not attributed to the grammar crate.

| Component | Version | License | Source |
| --- | --- | --- | --- |
| Tree-sitter | 0.26.11 | MIT | https://github.com/tree-sitter/tree-sitter |
| Tree-sitter highlight | 0.26.11 | MIT | https://github.com/tree-sitter/tree-sitter |
| Tree-sitter Rust grammar and queries | 0.24.2 | MIT | https://github.com/tree-sitter/tree-sitter-rust |
| Tree-sitter JavaScript grammar and queries | 0.25.0 | MIT | https://github.com/tree-sitter/tree-sitter-javascript |
| Tree-sitter TypeScript grammar and queries | 0.23.2 | MIT | https://github.com/tree-sitter/tree-sitter-typescript |
| Tree-sitter Python, Go, JSON, Bash, HTML, CSS, C, C++, C#, Java, Ruby and PHP grammars and queries | pinned in `Cargo.lock` | MIT | https://github.com/tree-sitter |
| Tree-sitter TOML, Markdown, YAML, Swift, SQL, Lua, Nix, Make and Containerfile grammars and queries | pinned in `Cargo.lock` | MIT-compatible; see each crate | Crate repositories recorded in `Cargo.lock` |
| Tree-sitter Kotlin grammar | 1.1.0 | MIT | https://github.com/tree-sitter-grammars/tree-sitter-kotlin |

Zeron also uses the following editor foundations from the pinned `zeronsh/gpui-component` fork. The fork aligns these crates with the same GPUI revision used by Comet.

| Component | Version | License | Source |
| --- | --- | --- | --- |
| gpui-base | 0.5.2 (`ed27327`) | Apache-2.0 | https://github.com/zeronsh/gpui-component |
| mermaid-rs-renderer | 0.3.1 | MIT | https://github.com/1jehuang/mermaid-rs-renderer |
| Ropey | 2.0.0-beta.1 | MIT | https://github.com/cessen/ropey |

The Linux windowing crate is vendored at `vendor/gpui_linux` from `zeronsh/zui` revision `c2d273dc3dadcb260b0fa7c35fc2fe02a14f5add`, with the Noches input repair described in `vendor/gpui_linux/NOCHES-PATCH.md`. That crate remains Apache-2.0; its upstream license is `vendor/gpui_linux/LICENSE-APACHE`.

Zeron's own source code is licensed under the terms in `LICENSE`. Bundled third-party components retain their respective licenses and notices.

## Symbols

Zeron bundles the SVG icon set and filename/folder associations from
[Symbols](https://github.com/miguelsolorio/vscode-symbols/tree/296ef1b62287fb2315cb5651e552e09e8c8e1de8).
Symbols is MIT licensed. The complete upstream license and copyright notice is
retained at `crates/ui/assets/file-icons/LICENSE.symbols`.

## Bundled theme palette adaptations

Zeron includes manually curated palette adaptations derived from the projects
below. The source repository and exact audited revision are also embedded in
each resolved theme variant. These projects are not affiliated with or endorsed
by Zeron. Their names identify the corresponding palette adaptations.

| Theme project | Audited revision | License and upstream notice |
| --- | --- | --- |
| Visual Studio Code Dark+/Light+ | `e33d147d4c0fa65ce17cb73ec9d798f064b4bf1f` | [MIT](https://github.com/microsoft/vscode/blob/e33d147d4c0fa65ce17cb73ec9d798f064b4bf1f/LICENSE.txt) |
| Catppuccin for VS Code | `befc9e6fc41980f4241408f7049755d47c06ff45` | [MIT](https://github.com/catppuccin/vscode/blob/befc9e6fc41980f4241408f7049755d47c06ff45/LICENSE) |
| Tokyo Night VS Code Theme | `7c0f11eaef322f293621ca7befe462214b7ea468` | [MIT](https://github.com/tokyo-night/tokyo-night-vscode-theme/blob/7c0f11eaef322f293621ca7befe462214b7ea468/LICENSE.txt) |
| Dracula for Visual Studio Code | `1b9ecf4d7e0c8cc2e2e890a7a41ad1db5fff1e6c` | [MIT](https://github.com/dracula/visual-studio-code/blob/1b9ecf4d7e0c8cc2e2e890a7a41ad1db5fff1e6c/LICENSE) |
| GitHub VS Code Theme | `cd78e5e4e7bcf132a6f428ae0f32264bb1b729cf` | [MIT](https://github.com/primer/github-vscode-theme/blob/cd78e5e4e7bcf132a6f428ae0f32264bb1b729cf/LICENSE) |
| Ayu for VS Code | `444ef92911cb75c3933c8003e3a7c79b6b6c914f` | [MIT](https://github.com/ayu-theme/vscode-ayu/blob/444ef92911cb75c3933c8003e3a7c79b6b6c914f/LICENSE) |
| Gruvbox Theme | `ca3b8ad203e84a884ca33fb84b5795cf43032709` | [MIT](https://github.com/jdinhify/vscode-theme-gruvbox/blob/ca3b8ad203e84a884ca33fb84b5795cf43032709/LICENSE) |
| Rosé Pine for VS Code | `d8f5ebe8e096fa833e997c07eb7685ee1677a4ba` | [MIT](https://github.com/rose-pine/vscode/blob/d8f5ebe8e096fa833e997c07eb7685ee1677a4ba/LICENSE) |
| Nord Visual Studio Code | `8ead09822c02d0d49d0f764104505e5a34d3689f` | [MIT](https://github.com/nordtheme/visual-studio-code/blob/8ead09822c02d0d49d0f764104505e5a34d3689f/license) |
| One Dark Pro | `e6ccf638d5b69aa38cd1005edb0ee7ba7ef6fedc` | [MIT](https://github.com/Binaryify/OneDark-Pro/blob/e6ccf638d5b69aa38cd1005edb0ee7ba7ef6fedc/LICENSE.txt) |
| Atom One Dark Theme | `a8be970644982221f9b61fb1c4b3da74b4beab79` | [MIT](https://github.com/akamud/vscode-theme-onedark/blob/a8be970644982221f9b61fb1c4b3da74b4beab79/LICENSE) |
| Night Owl | `cc291eba7976b20d7c66bde6883c27b902196b07` | [MIT](https://github.com/sdras/night-owl-vscode-theme/blob/cc291eba7976b20d7c66bde6883c27b902196b07/LICENSE.md) |
| Winter is Coming | `260547834cb6ac37dd5b8bb5842cc1c8d3164946` | [MIT](https://github.com/johnpapa/vscode-winteriscoming/blob/260547834cb6ac37dd5b8bb5842cc1c8d3164946/LICENSE.md) |
| Palenight Theme | `6291efaace90855abe3d79025327ca41b9a3138c` | [MIT](https://github.com/whizkydee/vscode-palenight-theme/blob/6291efaace90855abe3d79025327ca41b9a3138c/license.md) |
| SynthWave '84 | `ecfa2fe1279f7233663fa3f98a96e6756000567b` | [MIT](https://github.com/robb0wen/synthwave-vscode/blob/ecfa2fe1279f7233663fa3f98a96e6756000567b/LICENSE) |
| Shades of Purple | `e8eb49f33e5db05ceba6677367b33ddb27ad821c` | [MIT text with an additional “With condition” section](https://github.com/ahmadawais/shades-of-purple-vscode/blob/e8eb49f33e5db05ceba6677367b33ddb27ad821c/LICENSE.md); Zeron is MIT-licensed, satisfying the stated condition |
| Cobalt2 | `c4e9574372b85afad1682ed0fdd1ac0411c62512` | [MIT](https://github.com/wesbos/cobalt2-vscode/blob/c4e9574372b85afad1682ed0fdd1ac0411c62512/LICENSE) |
| Andromeda | `d1abb48c69493000aa0133a32d594eb25e523d4f` | [MIT](https://github.com/EliverLara/Andromeda/blob/d1abb48c69493000aa0133a32d594eb25e523d4f/LICENSE.md) |

The palette values are adapted under the corresponding upstream license. The
linked license pages contain each project's copyright and permission notice and
are pinned to the same revision as the adapted source.

Copyright notices retained from those pinned upstream licenses:

- Copyright (c) 2015 - present Microsoft Corporation
- Copyright (c) 2021 Catppuccin
- Copyright (c) 2018-present Enkia
- Copyright (c) 2016 Dracula Theme
- Copyright (c) 2020 Primer
- Copyright (c) 2016 Ike Kurghinyan
- Copyright © 2017 JD
- Copyright (c) 2021 Rosé Pine
- Copyright (c) 2016-present Sven Greb <development@svengreb.de> (https://www.svengreb.de)
- Copyright (c) 2013-2022 Binaryify
- Copyright (c) 2015 Mahmoud Ali
- Copyright (c) 2018 Sarah Drasner
- Copyright (c) 2015-2017 JohnPapa.net, LLC
- Copyright (c) 2017-present Olaolu Olawuyi
- Copyright (c) 2019 Robb Owen
- Copyright (c) 2015-∞ Ahmad Awais
- Copyright (c) 2018 Wes Bos, Roberto Achar
- Copyright (c) 2017 <eliverlara@gmail.com>

The common MIT permission notice for the adaptations above follows:

> Permission is hereby granted, free of charge, to any person obtaining a copy
> of this software and associated documentation files (the “Software”), to deal
> in the Software without restriction, including without limitation the rights
> to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
> copies of the Software, and to permit persons to whom the Software is
> furnished to do so, subject to the following conditions:
>
> The above copyright notice and this permission notice shall be included in
> all copies or substantial portions of the Software.
>
> THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
> IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
> FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
> AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
> LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
> OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
> SOFTWARE.

The pinned Shades of Purple license additionally says that anything built with
it should also be MIT licensed. Zeron is distributed under MIT terms.

## mermaid-rs-renderer

The pinned MIT-licensed renderer, version 0.3.1, is used for file previews and
chat diagrams. Its optional CLI and PNG features are disabled. Chat rendering
adds no third-party dependencies; generated SVG uses the existing GPUI/usvg
image preparation path.

MIT License

Copyright (c) 2026 mermaid-rs-renderer contributors

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

## Native browser host

The macOS browser uses [Wry 0.56.0](https://github.com/tauri-apps/wry/tree/wry-v0.56.0)
(MIT OR Apache-2.0) to host the system WebKit engine, with the `objc2` family
of bindings (MIT) and `block2` (MIT). Exact versions and transitive dependencies
are pinned in `Cargo.lock`. The browser integration is independently written
Zeron code.

The Zui native overlay renderer adapts Apache-2.0 GPUI code from
[`egoist/zed` at `57bd4fe`](https://github.com/egoist/zed/tree/57bd4fe181639797d395978d5de17bc9e10a6219/crates/gpui_macos).
Attribution is retained in the pinned Zui dependency’s `NOTICE`.

## Optional desktop dictation

NVIDIA Parakeet TDT 0.6B v3 model weights are licensed under CC BY 4.0.
The optional download uses Ivan Stupakov's INT8 ONNX conversion at immutable
revision `8f23f0c03c8761650bdb5b40aaf3e40d2c15f1ce`, verified by SHA-256.
Original: https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3.
Conversion: https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx.
License: https://creativecommons.org/licenses/by/4.0/.

Runtime dependencies are parakeet-rs 0.3.8 and ort/ort-sys 2.0.0-rc.13,
MIT OR Apache-2.0; ONNX Runtime 1.28.0, MIT; CPAL 0.15.3,
Apache-2.0; and rubato 0.16.2, MIT. Hound 3.5.1, Apache-2.0, reads WAVs
in the explicit verification example. Noches reuses CPAL 0.15.3 to preserve
the existing GPT-Live audio backend and its ALSA linkage.
See `crates/dictation/NOTICE.md` for model provenance and runtime limits.
The model is not bundled with the application.

The dictation port adds the following registry packages to the lockfile.
Existing package versions are unchanged. Sources and packaged license texts
are in each release's crate archive at `https://crates.io/crates/<name>/<version>`.
Slash-separated MIT/Apache declarations below offer either license.

| Added package and version | Declared license |
| --- | --- |
| base64 0.13.1 | MIT OR Apache-2.0 |
| castaway 0.2.4 | MIT |
| compact_str 0.9.1 | MIT |
| daachorse 3.0.3 | MIT OR Apache-2.0 |
| dary_heap 0.3.9 | MIT OR Apache-2.0 |
| der 0.8.2 | Apache-2.0 OR MIT |
| derive_builder, derive_builder_core, derive_builder_macro 0.20.2 | MIT OR Apache-2.0 |
| esaxx-rs 0.1.10 | Apache-2.0 |
| eyre 0.6.14 | MIT OR Apache-2.0 |
| foreign-types 0.3.2, foreign-types-shared 0.1.1 | MIT OR Apache-2.0 |
| hmac-sha256 1.1.15 | ISC |
| hound 3.5.1 | Apache-2.0 |
| indenter 0.3.4 | MIT OR Apache-2.0 |
| lzma-rust2 0.15.8 | Apache-2.0 |
| macro_rules_attribute, macro_rules_attribute-proc_macro 0.2.3 | Apache-2.0 OR MIT OR Zlib |
| matrixmultiply 0.3.11 | MIT OR Apache-2.0 |
| monostate, monostate-impl 0.1.18 | MIT OR Apache-2.0 |
| native-tls 0.2.18 | MIT OR Apache-2.0 |
| ndarray 0.17.2 | MIT OR Apache-2.0 |
| onig 6.5.3, onig_sys 69.9.3 | MIT |
| openssl 0.10.81 | Apache-2.0 |
| openssl-macros 0.1.1 | MIT OR Apache-2.0 |
| openssl-probe 0.2.1 | MIT OR Apache-2.0 |
| openssl-sys 0.9.117 | MIT |
| ort, ort-sys 2.0.0-rc.13 | MIT OR Apache-2.0 |
| parakeet-rs 0.3.8 | MIT OR Apache-2.0 |
| pastey 0.2.3 | MIT OR Apache-2.0 |
| pem-rfc7468 1.0.0 | Apache-2.0 OR MIT |
| primal-check 0.3.4 | MIT OR Apache-2.0 |
| rawpointer 0.2.1 | MIT OR Apache-2.0 |
| rayon-cond 0.4.0 | Apache-2.0 OR MIT |
| realfft 3.5.0 | MIT |
| rubato 0.16.2 | MIT |
| rustfft 6.4.1 | MIT OR Apache-2.0 |
| schannel 0.1.29 | MIT |
| security-framework 3.7.0, security-framework-sys 2.17.0 | MIT OR Apache-2.0 |
| spm_precompiled 0.1.4 | Apache-2.0 |
| strength_reduce 0.2.4 | MIT OR Apache-2.0 |
| tokenizers 0.23.2 | Apache-2.0 |
| transpose 0.2.3 | MIT OR Apache-2.0 |
| unicode-normalization-alignments 0.1.12 | MIT OR Apache-2.0 |
| unicode_categories 0.1.1 | MIT OR Apache-2.0 |
| webpki-root-certs 1.0.9 | CDLA-Permissive-2.0 |

The ONNX build script downloads a checksum-verified native archive from
`cdn.pyke.io` during compilation, not application startup. Linux x86_64's
ONNX Runtime static archive is 105,481,448 bytes before final linking and
dead-code elimination. The release binary size delta has not been measured.
Native TLS dependencies serve that build-time downloader: OpenSSL on Linux,
Security.framework on macOS and Schannel on Windows. ONNX also needs a C/C++
toolchain; Oniguruma builds its bundled C engine. No new PulseAudio requirement
is introduced. CPAL's ALSA runtime dependency was already present in GPT-Live.
Intel macOS needs a separately built ONNX Runtime 1.28.0 because the pinned
distribution has no prebuilt archive for it. macOS arm64, Linux x86_64/aarch64
and Windows x86_64 have upstream archives; Windows also links DirectML/DX12.
An archive is also available for Windows aarch64, not tested here.
These targets, permission prompts and native packaging still need verification
outside this Linux x86_64 build.

The tokenizer also statically bundles Oniguruma's C engine under BSD-2-Clause.
Its copyright, conditions and disclaimer, along with ONNX Runtime's MIT
notice, are retained in `crates/dictation/NOTICE.md` and copied into packages.
