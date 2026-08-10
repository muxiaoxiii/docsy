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
            | '\u{3400}'..='\u{4DBF}'
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
    matches!(
        character,
        '\u{FB1D}'..='\u{FB4F}' | '\u{FB50}'..='\u{FDFF}' | '\u{FE70}'..='\u{FEFE}'
    )
}

/// Normalize decoded text for comparison only: strip control characters,
/// decompose RTL presentation forms via NFKC, expand Latin ligatures, remove
/// invisible and bidi-control characters, and convert visual-order RTL text
/// back to logical order. The raw PDF operation remains unchanged.
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
            // Invisible format characters and bidi controls.
            '\u{00AD}' | '\u{034F}' | '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}'
            | '\u{2060}'..='\u{2064}' | '\u{2066}'..='\u{2069}' | '\u{FEFF}' => {}
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

/// Swap bracket-like characters when a visual-order RTL run is reversed, so
/// `)text(` becomes `(text)` instead of staying mirrored.
fn mirror_bracket(character: char) -> char {
    match character {
        '(' => ')',
        ')' => '(',
        '[' => ']',
        ']' => '[',
        '{' => '}',
        '}' => '{',
        '<' => '>',
        '>' => '<',
        '\u{0F3A}' => '\u{0F3B}',
        '\u{0F3B}' => '\u{0F3A}',
        '\u{0F3C}' => '\u{0F3D}',
        '\u{0F3D}' => '\u{0F3C}',
        '\u{2045}' => '\u{2046}',
        '\u{2046}' => '\u{2045}',
        '\u{207D}' => '\u{207E}',
        '\u{207E}' => '\u{207D}',
        '\u{208D}' => '\u{208E}',
        '\u{208E}' => '\u{208D}',
        '\u{3008}'..='\u{3011}' if (character as u32).is_multiple_of(2) => {
            char::from_u32(character as u32 + 1).unwrap_or(character)
        }
        '\u{3008}'..='\u{3011}' => char::from_u32(character as u32 - 1).unwrap_or(character),
        _ => character,
    }
}

fn is_combining_mark(character: char) -> bool {
    unicode_normalization::char::canonical_combining_class(character) != 0
}

/// A left-to-right cluster starts with a digit of any script (numbers stay
/// LTR inside RTL text) or a non-RTL letter.
fn cluster_is_ltr(cluster: &str) -> bool {
    cluster.chars().next().is_some_and(|base| {
        base.is_numeric() || (base.is_alphabetic() && !is_rtl_char(base))
    })
}

fn reverse_visual_rtl(text: &str) -> String {
    // Group characters into clusters of a base character plus its combining
    // marks, so diacritics (Hebrew points, Arabic vowel signs) stay attached
    // to their base character when the run is reversed.
    let mut clusters = Vec::<String>::new();
    for character in text.chars() {
        if is_combining_mark(character) && !clusters.is_empty() {
            if let Some(last) = clusters.last_mut() {
                last.push(character);
            }
        } else {
            clusters.push(character.to_string());
        }
    }

    if !clusters.iter().any(|cluster| cluster_is_ltr(cluster)) {
        return clusters
            .iter()
            .rev()
            .flat_map(|cluster| cluster.chars().map(mirror_bracket))
            .collect();
    }

    let is_ltr_cluster = |index: usize| {
        let base = clusters[index].chars().next().unwrap_or_default();
        if base.is_alphanumeric() {
            return cluster_is_ltr(&clusters[index]);
        }
        // Neutral characters (spaces, punctuation) inherit the direction of
        // an adjacent LTR cluster.
        (index > 0 && cluster_is_ltr(&clusters[index - 1]))
            || (index + 1 < clusters.len() && cluster_is_ltr(&clusters[index + 1]))
    };

    let mut runs = Vec::<(bool, Vec<&str>)>::new();
    let mut index = 0;
    while index < clusters.len() {
        let is_ltr = is_ltr_cluster(index);
        let mut run = Vec::<&str>::new();
        while index < clusters.len() && is_ltr_cluster(index) == is_ltr {
            run.push(&clusters[index]);
            index += 1;
        }
        runs.push((is_ltr, run));
    }

    runs.reverse();
    let mut result = String::with_capacity(text.len());
    for (is_ltr, run) in runs {
        if is_ltr {
            for cluster in run {
                result.push_str(cluster);
            }
        } else {
            for cluster in run.iter().rev() {
                result.extend(cluster.chars().map(mirror_bracket));
            }
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
                // Fullwidth ASCII variants (Ａ-Ｚ, ａ-ｚ, ０-９, fullwidth punctuation).
                '\u{FF01}'..='\u{FF5E}' => {
                    char::from_u32(character as u32 - 0xFEE0).unwrap_or(character)
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
        // Visual-order Arabic presentation forms [meem, hah] decompose and
        // reverse to the logical string "\u{062D}\u{0645}".
        assert_eq!(expand_ligatures("\u{FEE1}\u{FEA2}"), "\u{062D}\u{0645}");
    }

    #[test]
    fn normalizes_hebrew_presentation_forms() {
        // HEBREW LETTER YOD WITH HIRIQ decomposes to yod + combining hiriq,
        // and the combining mark stays after its base character.
        assert_eq!(expand_ligatures("\u{FB1D}"), "\u{05D9}\u{05B4}");
    }

    #[test]
    fn keeps_combining_marks_on_base_when_reversing() {
        // Visual order: final-mem, lamed+holam, vav, shin+qamats.
        let visual = "\u{05DD}\u{05DC}\u{05B9}\u{05D5}\u{05E9}\u{05B8}";
        assert_eq!(
            reverse_visual_rtl(visual),
            "\u{05E9}\u{05B8}\u{05D5}\u{05DC}\u{05B9}\u{05DD}"
        );
    }

    #[test]
    fn mirrors_brackets_when_reversing() {
        // Extracted visual-order text keeps the original bracket codepoints;
        // reversing must mirror them back into place.
        assert_eq!(
            reverse_visual_rtl("(\u{05D1}\u{05D0})"),
            "(\u{05D0}\u{05D1})"
        );
    }

    #[test]
    fn keeps_digit_runs_in_logical_order_when_reversing() {
        // Visual order of the logical string "אב12": digits stay an LTR run.
        assert_eq!(
            reverse_visual_rtl("12\u{05D1}\u{05D0}"),
            "\u{05D0}\u{05D1}12"
        );
        // Arabic-Indic digits behave the same way instead of being flipped.
        assert_eq!(
            reverse_visual_rtl("\u{0661}\u{0662}\u{05D1}\u{05D0}"),
            "\u{05D0}\u{05D1}\u{0661}\u{0662}"
        );
    }

    #[test]
    fn removes_bidi_control_characters() {
        assert_eq!(
            expand_ligatures("a\u{200E}\u{200F}\u{202E}\u{2066}b"),
            "ab"
        );
    }

    #[test]
    fn normalizes_fullwidth_ascii_for_matching() {
        assert_eq!(normalize_for_match("Ｈｅａｄｅｒ２０２４"), "Header2024");
    }
}
