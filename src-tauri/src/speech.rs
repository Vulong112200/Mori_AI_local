//! Cắt câu từ stream LLM để đọc dần (SPEC §7.3): câu đầu đi TTS ngay khi xong, không đợi cả câu trả lời.
//! Bỏ emoji, ký hiệu markdown và khối code (đọc code thành tiếng vô nghĩa).

/// Câu quá dài không có dấu kết thúc → cắt ở dấu phẩy/khoảng trắng cuối để TTS bắt đầu sớm.
const MAX_SENTENCE: usize = 220;
/// Câu đầu: người dùng đang chờ tiếng (đo: câu đầu 71 ký tự mất 1.8 s, 123 ký tự mất 2.6 s) → cắt ở dấu phẩy khi đủ dài,
/// và giới hạn ngắn hơn.
const FIRST_MIN: usize = 30;
const FIRST_MAX: usize = 50;

#[derive(Default)]
pub struct Splitter {
    buf: String,
    in_code: bool,
    /// Đã trả câu đầu.
    started: bool,
}

impl Splitter {
    /// Thêm một đoạn delta, trả các câu đã hoàn chỉnh (đã làm sạch, bỏ câu rỗng).
    pub fn push(&mut self, delta: &str) -> Vec<String> {
        self.buf.push_str(delta);
        let mut out = Vec::new();
        while let Some(end) = self.boundary() {
            let piece: String = self.buf.drain(..end).collect();
            self.take(&piece, &mut out);
            self.started |= !out.is_empty();
        }
        out
    }

    /// Hết stream: trả phần còn lại.
    pub fn finish(&mut self) -> Vec<String> {
        let rest = std::mem::take(&mut self.buf);
        let mut out = Vec::new();
        self.take(&rest, &mut out);
        out
    }

    /// Vị trí (byte) ngay sau ranh giới câu đầu tiên, nếu đã chắc chắn.
    fn boundary(&self) -> Option<usize> {
        let max = if self.started {
            MAX_SENTENCE
        } else {
            FIRST_MAX
        };
        let mut it = self.buf.char_indices().enumerate().peekable();
        while let Some((n, (i, c))) = it.next() {
            let end = i + c.len_utf8();
            match c {
                '\n' => return Some(end),
                ',' if !self.started && n >= FIRST_MIN => match it.peek() {
                    Some((_, (_, n))) if n.is_whitespace() => return Some(end),
                    Some(_) => {}
                    None => return None,
                },
                '.' | '?' | '!' | '…' | ':' | ';' => match it.peek() {
                    Some((_, (_, n))) if n.is_whitespace() => return Some(end),
                    Some(_) => {}        // "3.5", "package.json"
                    None => return None, // chưa biết ký tự sau → đợi thêm
                },
                _ => {}
            }
        }
        if let Some((limit, _)) = self.buf.char_indices().nth(max) {
            // Cắt ở dấu phẩy/khoảng trắng cuối trong `max` ký tự đầu; không có thì cắt cứng.
            let cut = self.buf[..limit]
                .rfind([',', ' '])
                .filter(|&p| p > 0)
                .map_or(limit, |p| p + 1);
            return Some(cut);
        }
        None
    }

    fn take(&mut self, piece: &str, out: &mut Vec<String>) {
        for line in piece.split_inclusive('\n') {
            if line.trim_start().starts_with("```") {
                self.in_code = !self.in_code;
                continue;
            }
            if self.in_code {
                continue;
            }
            let s = clean(line);
            if s.chars().any(char::is_alphanumeric) {
                out.push(s);
            }
        }
    }
}

/// Bỏ emoji + ký hiệu markdown, gộp khoảng trắng.
pub fn clean(s: &str) -> String {
    let t = s.trim_start();
    let t = t
        .strip_prefix("- ")
        .or_else(|| t.strip_prefix("* "))
        .or_else(|| t.strip_prefix("• "))
        .unwrap_or(t)
        .trim_start_matches('#');
    let kept: String = t
        .chars()
        .filter(|&c| !is_emoji(c) && !matches!(c, '*' | '`' | '#' | '_' | '~' | '>' | '|'))
        .collect();
    kept.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_emoji(c: char) -> bool {
    matches!(c as u32,
        0x1F000..=0x1FAFF   // emoji, ký hiệu, biểu tượng cảm xúc
        | 0x2600..=0x27BF   // ký hiệu linh tinh, dingbats
        | 0x2B00..=0x2BFF
        | 0xFE00..=0xFE0F   // variation selectors
        | 0x200D) // zero-width joiner
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all(deltas: &[&str]) -> Vec<String> {
        let mut s = Splitter::default();
        let mut out: Vec<String> = deltas.iter().flat_map(|d| s.push(d)).collect();
        out.extend(s.finish());
        out
    }

    #[test]
    fn splits_streamed_sentences_in_order() {
        let out = all(&["Xin chào", " bạn! Mình là", " Mori. Bạn cần", " gì?"]);
        assert_eq!(out, ["Xin chào bạn!", "Mình là Mori.", "Bạn cần gì?"]);
    }

    #[test]
    fn first_sentence_released_before_stream_ends() {
        let mut s = Splitter::default();
        assert!(s.push("Chào bạn.").is_empty()); // chưa biết ký tự sau dấu chấm
        assert_eq!(s.push(" Hôm"), ["Chào bạn."]);
    }

    #[test]
    fn keeps_decimals_and_filenames() {
        assert_eq!(
            all(&["Mở package.json, bản 3.5 nhé."]),
            ["Mở package.json, bản 3.5 nhé."]
        );
    }

    #[test]
    fn strips_emoji_markdown_and_code() {
        let out = all(&["**Chào** bạn 😊\n- Ý một\n```rust\nfn main() {}\n```\nXong rồi 👍"]);
        assert_eq!(out, ["Chào bạn", "Ý một", "Xong rồi"]);
    }

    #[test]
    fn long_text_without_punctuation_is_cut() {
        let long = "chữ ".repeat(80);
        let out = all(&[&long]);
        assert!(out.len() >= 2 && out.iter().all(|s| s.chars().count() <= MAX_SENTENCE + 4));
        assert_eq!(out.join(" "), long.trim());
    }

    #[test]
    fn first_sentence_cut_early_at_comma_then_normal() {
        let out = all(&[
            "Lịch sử máy tính bắt đầu từ rất lâu, khi con người dùng bàn tính, rồi đến máy cơ khí. Sau đó, điện tử ra đời.",
        ]);
        assert_eq!(
            out,
            [
                "Lịch sử máy tính bắt đầu từ rất lâu,",
                "khi con người dùng bàn tính, rồi đến máy cơ khí.",
                "Sau đó, điện tử ra đời."
            ]
        );
        assert_eq!(
            all(&["Ừ, mình đây."]),
            ["Ừ, mình đây."],
            "phẩy sớm quá → không cắt"
        );
        let long = "chữ ".repeat(40);
        assert!(all(&[&long])[0].chars().count() <= FIRST_MAX);
    }

    #[test]
    fn emoji_only_sentence_dropped() {
        assert!(all(&["😊😊\n"]).is_empty());
    }
}
