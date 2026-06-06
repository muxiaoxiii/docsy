//! docx 处理公共工具函数

/// XML 特殊字符转义
pub fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// 修复 WPS 产生的嵌套 `<w:p>` 结构。
///
/// WPS 有时会在 `<w:r>` 内插入 `<w:p>` 元素（非法 OOXML），导致 Word 打开时报错。
/// 本函数将嵌套的 `<w:p>...</w:p>` 解开为纯内容（去掉 `<w:p>` 开闭标签和 pPr），
/// 使其成为合法的 run 内容。
pub fn flatten_nested_paragraphs(xml: &str) -> String {
    let mut result = xml.to_string();
    for _ in 0..10 {
        let mut changed = false;
        let mut out = String::with_capacity(result.len());
        let mut i = 0;
        let bytes = result.as_bytes();
        let len = bytes.len();

        while i < len {
            if bytes[i] == b'<'
                && i + 4 < len
                && &bytes[i..i + 4] == b"<w:p"
                && (bytes[i + 4] == b'>' || bytes[i + 4] == b' ')
            {
                let first_end = result[i..].find('>').map(|p| i + p + 1).unwrap_or(len);
                let mut j = first_end;
                while j < len
                    && (bytes[j] == b' '
                        || bytes[j] == b'\n'
                        || bytes[j] == b'\r'
                        || bytes[j] == b'\t')
                {
                    j += 1;
                }
                if j + 4 < len
                    && bytes[j] == b'<'
                    && &bytes[j..j + 4] == b"<w:p"
                    && (bytes[j + 4] == b'>' || bytes[j + 4] == b' ')
                {
                    let inner_start = j;
                    let inner_open_end = result[j..].find('>').map(|p| j + p + 1).unwrap_or(len);
                    let mut depth = 1;
                    let mut k = inner_open_end;
                    while k < len && depth > 0 {
                        if bytes[k] == b'<'
                            && k + 4 < len
                            && &bytes[k..k + 4] == b"<w:p"
                            && (bytes[k + 4] == b'>' || bytes[k + 4] == b' ')
                        {
                            depth += 1;
                            k += result[k..].find('>').map(|p| p + 1).unwrap_or(len - k);
                        } else if bytes[k] == b'<' && k + 6 <= len && &bytes[k..k + 6] == b"</w:p>"
                        {
                            depth -= 1;
                            k += 6;
                        } else {
                            let ch = result[k..].chars().next().unwrap();
                            k += ch.len_utf8();
                        }
                    }
                    let outer_end = if k + 6 <= len && &bytes[k..k + 6] == b"</w:p>" {
                        k + 6
                    } else {
                        k
                    };
                    out.push_str(&result[inner_start..k]);
                    i = outer_end;
                    changed = true;
                    continue;
                }
            }
            let ch = result[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
        result = out;
        if !changed {
            break;
        }
    }
    result
}
