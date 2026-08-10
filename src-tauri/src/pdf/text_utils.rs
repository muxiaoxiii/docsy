//! Conservative text helpers shared by PDF extraction and editing.
//!
//! These helpers normalize only decoded text used for comparison. They never
//! rewrite the PDF content stream because normalized text does not prove that
//! a source glyph range can be edited safely.

use unicode_normalization::UnicodeNormalization;

pub(crate) fn is_cjk_char(character: char) -> bool {
    matches!(
        character,
        '\u{1100}'..='\u{11FF}'
            | '\u{3000}'..='\u{303F}'
            | '\u{3040}'..='\u{30FF}'
            | '\u{3130}'..='\u{318F}'
            | '\u{4E00}'..='\u{9FFF}'
            | '\u{AC00}'..='\u{D7AF}'
            | '\u{F900}'..='\u{FAFF}'
            | '\u{FF00}'..='\u{FFEF}'
    )
}

fn is_rtl_char(character: char) -> bool {
    matches!(
        character,
        '\u{0590}'..='\u{05FF}'
            | '\u{0600}'..='\u{06FF}'
            | '\u{0700}'..='\u{074F}'
            | '\u{0750}'..='\u{077F}'
            | '\u{0780}'..='\u{07BF}'
            | '\u{07C0}'..='\u{07FF}'
            | '\u{0800}'..='\u{083F}'
            | '\u{0840}'..='\u{085F}'
            | '\u{08A0}'..='\u{08FF}'
            | '\u{FB1D}'..='\u{FB4F}'
            | '\u{FB50}'..='\u{FDFF}'
            | '\u{FE70}'..='\u{FEFE}'
    )
}

fn is_rtl_text(text: &str) -> bool {
    let (mut rtl, mut ltr) = (0_u32, 0_u32);
    for character in text.chars() {
        if is_rtl_char(character) {
            rtl += 1;
        } else if character.is_alphabetic() && !is_cjk_char(character) {
            ltr += 1;
        }
    }
    rtl > 0 && rtl > ltr
}

fn is_rtl_presentation_form(character: char) -> bool {
    matches!(character, '\u{FB50}'..='\u{FDFF}' | '\u{FE70}'..='\u{FEFE}')
}

/// Expand common Unicode ligatures and remove invisible characters for text
/// comparison. The raw PDF operation remains unchanged.
pub(crate) fn expand_ligatures(text: &str) -> String {
    let cleaned: String = text
        .chars()
        .filter(|character| {
            let code = *character as u32;
            code >= 0x20 || matches!(*character, '\n' | '\r' | '\t')
        })
        .collect();
    let had_rtl_presentation_forms = cleaned.chars().any(is_rtl_presentation_form);
    let normalized = if had_rtl_presentation_forms {
        cleaned.nfkc().collect::<String>()
    } else {
        cleaned
    };

    let mut result = String::with_capacity(normalized.len());
    for character in normalized.chars() {
        match character {
            '\u{FB00}' => result.push_str("ff"),
            '\u{FB01}' => result.push_str("fi"),
            '\u{FB02}' => result.push_str("fl"),
            '\u{FB03}' => result.push_str("ffi"),
            '\u{FB04}' => result.push_str("ffl"),
            '\u{FB05}' | '\u{FB06}' => result.push_str("st"),
            '\u{00AD}' | '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}' | '\u{FEFF}' => {}
            '\u{2000}'..='\u{200A}' => result.push(' '),
            _ => result.push(character),
        }
    }

    if had_rtl_presentation_forms && is_rtl_text(&result) {
        reverse_visual_rtl(&result)
    } else {
        result
    }
}

fn reverse_visual_rtl(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let has_ltr = chars
        .iter()
        .any(|character| character.is_ascii_alphanumeric());
    if !has_ltr {
        return chars.into_iter().rev().collect();
    }

    fn adjacent_to_ascii_alnum(chars: &[char], index: usize) -> bool {
        (index > 0 && chars[index - 1].is_ascii_alphanumeric())
            || (index + 1 < chars.len() && chars[index + 1].is_ascii_alphanumeric())
    }

    let mut runs = Vec::<(bool, String)>::new();
    let mut index = 0;
    while index < chars.len() {
        let is_ltr = chars[index].is_ascii_alphanumeric()
            || (chars[index].is_ascii_punctuation() && adjacent_to_ascii_alnum(&chars, index));
        let mut run = String::new();
        while index < chars.len() {
            let character = chars[index];
            let current_is_ltr = character.is_ascii_alphanumeric()
                || (character.is_ascii_punctuation() && adjacent_to_ascii_alnum(&chars, index));
            if current_is_ltr != is_ltr {
                break;
            }
            run.push(character);
            index += 1;
        }
        runs.push((is_ltr, run));
    }

    runs.reverse();
    let mut result = String::with_capacity(text.len());
    for (is_ltr, run) in runs {
        if is_ltr {
            result.push_str(&run);
        } else {
            result.extend(run.chars().rev());
        }
    }
    result
}

/// Normalize only the comparison representation. Source bytes and operation
/// boundaries remain unchanged for all edit operations.
pub(crate) fn normalize_for_match(text: &str) -> String {
    expand_ligatures(text)
        .chars()
        .filter_map(|character| {
            let normalized = match character {
                '０'..='９' => {
                    char::from_u32(character as u32 - '０' as u32 + '0' as u32).unwrap_or(character)
                }
                _ => character,
            };
            (!normalized.is_whitespace()).then_some(normalized)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_latin_ligatures_and_invisible_characters() {
        assert_eq!(expand_ligatures("of\u{FB01}ce\u{200B}"), "office");
    }

    #[test]
    fn keeps_cjk_and_normal_arabic_in_logical_order() {
        assert!(is_cjk_char('证'));
        assert_eq!(expand_ligatures("你好"), "你好");
        assert_eq!(expand_ligatures("مرحبا"), "مرحبا");
    }

    #[test]
    fn normalizes_ligatures_for_matching_only() {
        assert_eq!(normalize_for_match("  of\u{FB01}ce  "), "office");
    }

    #[test]
    fn normalizes_rtl_presentation_forms() {
        let value = expand_ligatures("\u{FEE1}\u{FEF3}");
        assert!(value
            .chars()
            .all(|character| !is_rtl_presentation_form(character)));
        assert!(value.chars().any(is_rtl_char));
    }
}
