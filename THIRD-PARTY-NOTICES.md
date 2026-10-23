# 第三方组件声明

本产品包含以下第三方开源组件，按各自许可证使用。

## Docsy AnyDoc（基于 Firecrawl AnyDoc 0.2.4 的修改版）

- 来源：vendored 于 `src-tauri/vendor/docsy-anydoc`
- 上游项目：https://github.com/firecrawl/anydoc
- 许可证：MIT License
- 版权：Copyright (c) 2026 Sideguide Technologies Inc.

本组件用于将 Office / OpenDocument / RTF / CSV / EPUB 等文档提取为 Markdown，
Docsy 在其 0.2.4 版本基础上做了适配性修改。许可证全文如下（英文原文）：

```text
MIT License

Copyright (c) 2026 Sideguide Technologies Inc.

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
```

## doc2x（旧版 `.doc` → `.docx` 侧车转换器）

- 来源：`src-tauri/runtime/bin/doc2x`（由 `scripts/prepare-doc2x.mjs` 按平台钉版本准备）
- 上游包装：https://github.com/jitOffice/doc2docx
- 派生自：b2xtranslator（BSD-3-Clause）
- 许可证：BSD-3-Clause（全文见 `src-tauri/runtime/licenses/doc2x-BSD-3-Clause.txt`）
- 溯源：`src-tauri/runtime/licenses/doc2x-provenance.json`

仅用于文书模板等场景把旧版 Word `.doc` 转成 `.docx`；Docsy 不重实现 MS-DOC 解析。

## htmd（HTML → Markdown 转换库）

- 来源：crates.io 依赖 `htmd`（0.5.x）
- 上游项目：https://github.com/letmutex/htmd
- 许可证：Apache License 2.0

用于 MD 转换模块把 HTML 文件提取为 Markdown（语义结构，不执行脚本）。

## opencv-mobile（精简嵌入式 OpenCV 构建）

- 来源：vendored 于 `src-tauri/vendor/opencv-mobile`
- 上游项目：https://github.com/nihui/opencv-mobile
- 许可证：Apache License 2.0
- 版权：Copyright (C) 2000-2023, Intel Corporation, OpenCV Foundation, Willow Garage Inc.

本组件仅供本地视频抽帧画面增强与智能选图使用，以静态库方式内嵌，无需用户单独配置 OpenCV 环境。
许可证全文如下（英文原文）：

```text
                                 Apache License
                           Version 2.0, January 2004
                        http://www.apache.org/licenses/

   TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

   1. Definitions.

      "License" shall mean the terms and conditions for use, reproduction,
      and distribution as defined by Sections 1 through 9 of this document.

      "Licensor" shall mean the copyright owner or entity authorized by
      the copyright owner that is granting the License.

      "Legal Entity" shall mean the union of the acting entity and all
      other entities that control, are controlled by, or are under common
      control with that entity. For the purposes of this definition,
      "control" means (i) the power, direct or indirect, to cause the
      direction or management of such entity, whether by contract or
      otherwise, or (ii) ownership of fifty percent (50%) or more of the
      outstanding shares, or (iii) beneficial ownership of such entity.

      "You" (or "Your") shall mean an individual or Legal Entity
      exercising permissions granted by this License.

      "Source" form shall mean the preferred form for making modifications,
      including but not limited to software source code, documentation
      source, and configuration files.

      "Object" form shall mean any form resulting from mechanical
      transformation or translation of a Source form, including but
      not limited to compiled object code, generated documentation,
      and conversions to other media types.

      "Work" shall mean the work of authorship, whether in Source or
      Object form, made available under the License, as indicated by a
      copyright notice that is included in or attached to the work.

      "Derivative Works" shall mean any work, whether in Source or Object
      form, that is based on (or derived from) the Work and for which the
      editorial revisions, annotations, elaborations, or other modifications
      represent, as a whole, an original work of authorship. For the purposes
      of this License, Derivative Works shall not include works that remain
      separable from, or merely link (or bind by name) to the interfaces of,
      the Work and Derivative Works thereof.

      "Contribution" shall mean any work of authorship, including
      the original version of the Work and any modifications or additions
      to that Work or Derivative Works thereof, that is intentionally
      submitted to Licensor for inclusion in the Work by the copyright owner
      or by an individual or Legal Entity authorized to submit on behalf of
      the copyright owner.

      "Contributor" shall mean Licensor and any individual or Legal Entity
      on behalf of whom a Contribution has been received by Licensor and
      subsequently incorporated within the Work.

   2. Grant of Copyright License. Subject to the terms and conditions of
      this License, each Contributor hereby grants to You a perpetual,
      worldwide, non-exclusive, no-charge, royalty-free, irrevocable
      copyright license to reproduce, prepare Derivative Works of,
      publicly display, publicly perform, sublicense, and distribute the
      Work and such Derivative Works in Source or Object form.

   3. Grant of Patent License. Subject to the terms and conditions of
      this License, each Contributor hereby grants to You a perpetual,
      worldwide, non-exclusive, no-charge, royalty-free, irrevocable
      (except as stated in this section) patent license to make, have made,
      use, offer to sell, sell, import, and otherwise transfer the Work,
      where such license applies only to those patent claims licensable
      by such Contributor that are necessarily infringed by their
      Contribution(s) alone or by combination of their Contribution(s)
      with the Work to which such Contribution(s) was submitted.

   4. Redistribution. You may reproduce and distribute copies of the
      Work or Derivative Works thereof in any medium, with or without
      modifications, and in Source or Object form, provided that You
      meet the following conditions:

      (a) You must give any other recipients of the Work or
          Derivative Works a copy of this License; and

      (b) You must cause any modified files to carry prominent notices
          stating that You changed the files; and

      (c) You must retain, in the Source form of any Derivative Works
          that You distribute, all copyright, patent, trademark, and
          attribution notices from the Source form of the Work,
          excluding those notices that do not pertain to any part of
          the Derivative Works; and

      (d) If the Work includes a "NOTICE" text file as part of its
          distribution, then any Derivative Works that You distribute must
          include a readable copy of the attribution notices contained
          within such NOTICE file.

   5. Submission of Contributions. Unless You explicitly state otherwise,
      any Contribution intentionally submitted for inclusion in the Work
      by You to the Licensor shall be under the terms and conditions of
      this License, without any additional terms or conditions.

   6. Trademarks. This License does not grant permission to use the trade
      names, trademarks, service marks, or product names of the Licensor,
      except as required for reasonable and customary use in describing the
      origin of the Work and reproducing the content of the NOTICE file.

   7. Disclaimer of Warranty. Unless required by applicable law or
      agreed to in writing, Licensor provides the Work (and each
      Contributor provides its Contributions) on an "AS IS" BASIS,
      WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
      implied, including, without limitation, any warranties or conditions
      of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
      PARTICULAR PURPOSE.

   8. Limitation of Liability. In no event and under no legal theory,
      whether in tort (including negligence), contract, or otherwise,
      shall any Contributor be liable to You for damages, including any
      direct, indirect, special, incidental, or consequential damages of
      any character arising as a result of this License or out of the
      use or inability to use the Work.

   9. Accepting Warranty or Additional Liability. While redistributing
      the Work or Derivative Works thereof, You may choose to offer,
      and charge a fee for, acceptance of support, warranty, indemnity,
      or other liability obligations and/or rights consistent with this
      License.
```

## PdfCraft（基于 upstream commit `4db6ac4ae2448df6389d8bb0bc54db6ba175721c` 的修改版）

- 来源：vendored 于 `src-tauri/vendor/pdfcraft`
- 上游项目：https://github.com/storytold/pdfcraft
- 许可证：MIT License OR Apache License 2.0
- 版权：Copyright (c) 2024-2026 Storytold
- 详细说明与补丁记录：见 `src-tauri/vendor/pdfcraft/VENDOR.md`

用于 PDF 原生合并、单页与多页提取、元数据解析以及表单/批注外观压平。Docsy 包含以下本地安全及兼容性补丁：
1. `pdfcraft_cos`: 宽松解析兼容，损坏 xref/trailer 及缺少 startxref 标记容错恢复。
2. `pdfcraft_organize`: 修正 ISO 32000-1 规范下 UserUnit 属性非继承行为，防止页面尺寸与正文坐标错位。
3. `pdfcraft_organize`: 页面提取与合并时保留 NeedAppearances 字段，防止旧版表单外观固化。

许可证全文见上文 Apache License 2.0 及下述 MIT License 原文：

```text
MIT License

Copyright (c) 2024-2026 Storytold

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
```

## TuneShape 动效交互参考

- 来源：https://github.com/orocoa/TuneShape
- 参考快照：`ff7871f8dea7920115dce1f90f83e15eff86845a`
- 许可证：MIT；Copyright (c) 2026 TuneShape contributors

按钮按压、标签页和弹窗的交互状态与动效参数参考其公开示例，并以 Vue、CSS 和浏览器动画接口重新适配；未引入 React 或 Motion 依赖。许可随安装包的 `runtime/licenses/TuneShape-MIT.txt` 提供。

```text
MIT License

Copyright (c) 2026 TuneShape contributors

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
```
