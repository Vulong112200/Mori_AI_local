//! Cảm xúc của câu trả lời → biểu cảm nhân vật (Phase 7). Chỉ nhãn đi sang cửa sổ avatar, không có nội dung.
//! ponytail: dò emoji + vài từ khóa tiếng Việt; đổi sang để model gắn nhãn nếu sai nhiều.

/// "happy" | "love" | "sad" | "surprised" | "shy" | "worried" | "neutral".
pub fn of(text: &str) -> &'static str {
    let t = text.to_lowercase();
    let has = |chars: &str, words: &[&str]| t.chars().any(|c| chars.contains(c)) || words.iter().any(|w| t.contains(w));
    if has("😢😭🥺😞😔💔", &["buồn quá", "tiếc quá", "thương bạn", "tội nghiệp"]) {
        "sad"
    } else if has("😮😲😯🤯😱", &["trời ơi", "ôi chao", "thật á", "bất ngờ", "wow"]) {
        "surprised"
    } else if has("🥰😍❤💕💖😘", &["yêu bạn", "thích bạn", "quý bạn"]) {
        "love"
    } else if has("😳☺🙈", &["ngại quá", "xấu hổ", "mắc cỡ"]) {
        "shy"
    } else if has("😟😥😓😰🤔", &["lo quá", "cẩn thận", "đừng lo", "hmm"]) {
        "worried"
    } else if has("😊😄😆😁🎉✨😉😋🥳🤗👍", &["vui quá", "tuyệt", "hay quá", "haha", "hihi", "giỏi quá", "chúc mừng"]) {
        "happy"
    } else {
        "neutral"
    }
}

#[cfg(test)]
mod tests {
    use super::of;

    #[test]
    fn detects_common_emotions() {
        assert_eq!(of("Bạn làm kế toán à, nghe hay quá! 😊"), "happy");
        assert_eq!(of("Ôi, Mori tiếc quá 😢"), "sad");
        assert_eq!(of("Trời ơi, thật á?"), "surprised");
        assert_eq!(of("Mori quý bạn lắm 🥰"), "love");
        assert_eq!(of("Bạn nhớ cẩn thận khi ăn tôm nha"), "worried");
        assert_eq!(of("Thủ đô của Việt Nam là Hà Nội."), "neutral");
    }
}
