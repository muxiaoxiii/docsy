修复证据处理模块页眉页脚检测的误判问题。

## 现状问题
用真实案件文件测试（/Users/only/Desktop/Test/【5号专利证据分项】），
检测结果中大量正文内容被误判为页眉。

误判示例：
- ") 登记 日 局" — 企业信用报告的表格内容
- "经营范围变" — 表格标题
- "、机动车配件、装饰品、园" — 经营范围文本
- "• 2023年第四季度的毛利率為7.5%" — 财报正文
- "2022年 2023年 2023年" — 表格列标题

真正的页眉应该是"证据1"、"证据11"这种在每页固定位置的证据编号。

## 根因分析

1. header_zone_ratio = 0.12（12%页面高度 ≈ 35mm），太大。
   A4 页面页眉通常在 15mm 以内。缩小到 0.06（6% ≈ 17mm）。

2. position_stable 阈值 0.025（2.5%页面高度 ≈ 7mm），太宽松。
   正真页眉位置变化应在 1mm 以内。收紧到 0.008。

3. confidence 最低阈值未过滤。单页文件 confidence=0.25，
   多页文件如果 count/pages_analyzed > 0.3 就通过。
   应该过滤 confidence < 0.4 的候选。

4. 内容长度过滤不够。"、机动车配件、装饰品、园"这种长文本
   不太可能是页眉。页眉通常 < 30字符。

## 修改方案

### Rust 端 src-tauri/src/pdf/detection.rs

1. default_header_zone_ratio: 0.12 → 0.06
2. default_footer_zone_ratio: 0.12 → 0.06
3. position_stable 阈值: 0.025 → 0.008
4. build_candidates 末尾增加过滤：
   - confidence < 0.4 的 content-text 候选过滤掉
   - normalized_text 长度 > 40 字符的 content-text 候选过滤掉
5. artifact 来源的候选不受这些过滤影响（保持原样）

### 前端
已有低置信度过滤逻辑，不需要额外修改。

## 验证
cargo test + npm test + npm run build 全过。
