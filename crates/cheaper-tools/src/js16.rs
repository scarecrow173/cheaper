//! JS string の UTF-16 code unit セマンティクスを扱う共有 helper。
//!
//! `String#length`・`String#slice`・`split("")` は UTF-16 code unit 単位
//! (astral char は surrogate pair で 2 unit)で動く。Rust の `chars()`
//! (Unicode scalar value)や byte 長と混同しないよう、UTF-16 単位の
//! 計測・切断はこの module に集約する。

/// JS `String#length`(UTF-16 code unit 数)。
pub fn utf16_len(s: &str) -> usize {
    s.chars().map(char::len_utf16).sum()
}

/// JS `String#slice(0, n)` 相当: UTF-16 code unit で n までを返す
/// (astral char は 2 unit。unit 境界が char 内部に落ちる場合は手前で
/// 切る — surrogate 半端は UTF-8 で表現できないため)。
pub fn slice_utf16(s: &str, n: usize) -> &str {
    let mut units = 0usize;
    for (i, c) in s.char_indices() {
        let u = c.len_utf16();
        if units + u > n {
            return &s[..i];
        }
        units += u;
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_len_counts_astral_chars_as_two() {
        assert_eq!(utf16_len("abc"), 3);
        assert_eq!(utf16_len("😀"), 2);
        assert_eq!(utf16_len("a😀b"), 4);
    }

    #[test]
    fn slice_utf16_does_not_split_surrogate_pair() {
        // 1 unit では「😀」の前半 surrogate しか入らないため 0 char で切る。
        assert_eq!(slice_utf16("😀b", 1), "");
        assert_eq!(slice_utf16("😀b", 2), "😀");
        assert_eq!(slice_utf16("a😀b", 2), "a");
        assert_eq!(slice_utf16("a😀b", 3), "a😀");
        assert_eq!(slice_utf16("a😀b", 99), "a😀b");
        assert_eq!(slice_utf16("", 0), "");
    }
}
