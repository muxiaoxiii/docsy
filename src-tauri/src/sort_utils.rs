use std::cmp::Ordering;

pub fn natural_cmp(left: &str, right: &str) -> Ordering {
    let (mut left_index, mut right_index) = (0usize, 0usize);
    let (left_bytes, right_bytes) = (left.as_bytes(), right.as_bytes());
    while left_index < left_bytes.len() && right_index < right_bytes.len() {
        if left_bytes[left_index].is_ascii_digit() && right_bytes[right_index].is_ascii_digit() {
            let (left_end, right_end) = (
                ascii_digit_end(left_bytes, left_index),
                ascii_digit_end(right_bytes, right_index),
            );
            let left_trimmed = trim_leading_zeroes(left_bytes, left_index, left_end);
            let right_trimmed = trim_leading_zeroes(right_bytes, right_index, right_end);
            let left_len = left_end - left_trimmed;
            let right_len = right_end - right_trimmed;
            if left_len != right_len {
                return left_len.cmp(&right_len);
            }
            for offset in 0..left_len {
                let ordering =
                    left_bytes[left_trimmed + offset].cmp(&right_bytes[right_trimmed + offset]);
                if ordering != Ordering::Equal {
                    return ordering;
                }
            }
            let original_len_order = (left_end - left_index).cmp(&(right_end - right_index));
            if original_len_order != Ordering::Equal {
                return original_len_order;
            }
            left_index = left_end;
            right_index = right_end;
        } else {
            let ordering = left_bytes[left_index]
                .to_ascii_lowercase()
                .cmp(&right_bytes[right_index].to_ascii_lowercase());
            if ordering != Ordering::Equal {
                return ordering;
            }
            left_index += 1;
            right_index += 1;
        }
    }
    left_bytes.len().cmp(&right_bytes.len())
}

fn ascii_digit_end(bytes: &[u8], start: usize) -> usize {
    let mut end = start;
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    end
}

fn trim_leading_zeroes(bytes: &[u8], start: usize, end: usize) -> usize {
    let mut index = start;
    while index + 1 < end && bytes[index] == b'0' {
        index += 1;
    }
    index
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorts_embedded_numbers_naturally() {
        let mut values = vec!["证据10.pdf", "证据2.pdf", "证据1.pdf"];
        values.sort_by(|left, right| natural_cmp(left, right));
        assert_eq!(values, vec!["证据1.pdf", "证据2.pdf", "证据10.pdf"]);
    }

    #[test]
    fn handles_leading_zeroes_stably() {
        let mut values = vec!["img_001.jpg", "img_1.jpg", "img_010.jpg", "img_2.jpg"];
        values.sort_by(|left, right| natural_cmp(left, right));
        assert_eq!(
            values,
            vec!["img_1.jpg", "img_001.jpg", "img_2.jpg", "img_010.jpg"]
        );
    }
}
