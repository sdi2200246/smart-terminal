use super::Buffer;

impl Buffer{
    pub fn set_suggestion(&mut self, s: String) {
        self.suggestion_buffer = s;
    }

    pub fn merge_suggestion(&mut self){
        self.user_buffer.clear();
        self.user_buffer.push_str(&self.suggestion_buffer);
    }

    pub fn clear_buffer(&mut self){
        self.user_buffer.clear();
        self.suggestion_buffer.clear();
        self.cursor = 0;
    }
    
    pub fn buffer_suffix(&self) -> &str {
        self.suggestion_buffer
            .strip_prefix(&self.user_buffer)
            .unwrap_or("")
    }

    pub fn take_user_bytes(&mut self) -> Vec<u8> {
        let bytes = self.user_buffer.as_bytes().to_vec();
        self.user_buffer.clear();
        self.suggestion_buffer.clear();
        bytes
    }

    pub fn get_user_buffer(&self)->&str{
        &self.user_buffer
    }
    pub fn to_bytes(&self)-> Vec<u8> {
        let mut bytes = self.user_buffer.as_bytes().to_vec();
        bytes.push(b'\r');
        bytes
    }


    pub fn push(&mut self , c:&str){
        self.user_buffer.insert_str(self.cursor, c);
        self.cursor += c.len();
    }

    pub fn pop(&mut self){

        if self.cursor == 0 {return}
        let prev = &self.user_buffer[..self.cursor].char_indices().last().unwrap();        
        self.user_buffer.drain(prev.0 ..self.cursor);
        self.cursor = prev.0;

    }
    pub fn cursor_right(&mut self){
        if self.cursor >= self.user_buffer.len() {
            return;
        }
        let next = self.user_buffer[self.cursor..]
            .char_indices()
            .nth(1)  
            .map(|(i, _)| self.cursor + i)
            .unwrap_or(self.user_buffer.len());

        self.cursor = next;
    }
    pub fn cursor_left(&mut self){
        if self.cursor == 0 {
            return;
        }
        let prev = self.user_buffer[..self.cursor]
            .char_indices()
            .nth_back(0)                    
            .map(|(i, _)| i)
            .unwrap_or(0);

        self.cursor = prev;
    }

}

impl Default for Buffer{
    fn default()->Self{
        Self{
            user_buffer:String::new(),
            suggestion_buffer:String::new(),
            cursor:0,
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    mod suffix {
        use super::*;

        #[test]
        fn normal_suffix() {
            let buffer = Buffer{
                user_buffer:"git".to_string(),
                suggestion_buffer:"git_push".to_string(),
                cursor:0,
            };
            assert_eq!(buffer.buffer_suffix(), "_push");
        }

        #[test]
        fn no_common_prefix() {
            let buffer = Buffer{
                user_buffer:"git".to_string(),
                suggestion_buffer:"piss".to_string(),
                cursor:0,
            };
            assert_eq!(buffer.buffer_suffix(), "");
        }

        #[test]
        fn empty_user_buffer() {
            let buffer = Buffer{
                user_buffer:"".to_string(),
                suggestion_buffer:"piss".to_string(),
                cursor:0,
            };
            assert_eq!(buffer.buffer_suffix(), "piss");
        }
    }

    mod merge {
        use super::*;

        #[test]
        fn merges_full_suggestion() {
            let mut buffer = Buffer{
                user_buffer:"me".to_string(),
                suggestion_buffer:"mepiss".to_string(),
                cursor:0,
            };
            buffer.merge_suggestion();
            assert_eq!(buffer.get_user_buffer(), "mepiss");
        }
    }

    mod push {
        use super::*;

        #[test]
        fn sequential_pushes() {
            let mut buf = Buffer::default();

            buf.push("g");
            buf.push("it");
            buf.push(" status");

            assert_eq!(buf.get_user_buffer(), "git status");
        }

        #[test]
        fn push_at_start() {
            let mut buf = Buffer {
                user_buffer: "".to_string(),
                suggestion_buffer: "".to_string(),
                cursor: 0,
            };

            buf.push("git");
            assert_eq!(buf.get_user_buffer(), "git");
            assert_eq!(buf.cursor, 3);
        }

        #[test]
        fn push_in_middle() {
            let mut buf = Buffer {
                user_buffer: "gtt".to_string(),
                suggestion_buffer: "".to_string(),
                cursor: 1,
            };

            buf.push("i");
            assert_eq!(buf.get_user_buffer(), "gitt");
            assert_eq!(buf.cursor, 2);
        }

        #[test]
        fn push_utf8() {
            let mut buf = Buffer {
                user_buffer: "ab".to_string(),
                suggestion_buffer: "".to_string(),
                cursor: 2,
            };

            buf.push("ç");
            assert_eq!(buf.get_user_buffer(), "abç");
            assert_eq!(buf.cursor, "abç".len());
        }
    }

    mod pop {
        use super::*;

        #[test]
        fn pop_basic() {
            let mut buf = Buffer {
                user_buffer: "git".to_string(),
                suggestion_buffer: "".to_string(),
                cursor: 3,
            };

            buf.pop();
            assert_eq!(buf.get_user_buffer(), "gi");
            assert_eq!(buf.cursor, 2);
        }

        #[test]
        fn pop_whitechar() {
            let mut buf = Buffer {
                user_buffer: "hi\n".to_string(),
                suggestion_buffer: "".to_string(),
                cursor: "hi\n".len(),
            };

            buf.pop();
            assert_eq!(buf.get_user_buffer(), "hi");
            assert_eq!(buf.cursor, 2);
        }
        #[test]
        fn pop_can_move_cursor_to_zero() {
            let mut buf = Buffer {
                user_buffer: "abc".to_string(),
                suggestion_buffer: "".to_string(),
                cursor: 1,
            };

            buf.pop();

            assert_eq!(buf.get_user_buffer(), "bc");
            assert_eq!(buf.cursor, 0);
        }
    }

    #[cfg(test)]
    mod cursor_movement {
        use super::*;

        #[test]
        fn cursor_right_moves_by_one_character() {
            let mut buf = Buffer {
                user_buffer: "abc".to_string(),
                suggestion_buffer: "".to_string(),
                cursor: 0,
            };

            buf.cursor_right(); // |abc → a|bc
            assert_eq!(buf.cursor, 1);

            buf.cursor_right(); // a|bc → ab|c
            assert_eq!(buf.cursor, 2);

            buf.cursor_right(); // ab|c → abc|
            assert_eq!(buf.cursor, 3);

            buf.cursor_right(); // no-op at end
            assert_eq!(buf.cursor, 3);
        }

        #[test]
        fn cursor_left_moves_by_one_character() {
            let mut buf = Buffer {
                user_buffer: "abc".to_string(),
                suggestion_buffer: "".to_string(),
                cursor: 3,
            };

            buf.cursor_left(); // abc| → ab|c
            assert_eq!(buf.cursor, 2);

            buf.cursor_left(); // ab|c → a|bc
            assert_eq!(buf.cursor, 1);

            buf.cursor_left(); // a|bc → |abc
            assert_eq!(buf.cursor, 0);

            buf.cursor_left(); // no-op at start
            assert_eq!(buf.cursor, 0);
        }

        #[test]
        fn cursor_moves_utf8_safely() {
            let s = "aç😊b";
            let mut buf = Buffer {
                user_buffer: s.to_string(),
                suggestion_buffer: "".to_string(),
                cursor: 0,
            };

            // move right across UTF-8
            while buf.cursor < buf.user_buffer.len() {
                assert!(buf.user_buffer.is_char_boundary(buf.cursor));
                buf.cursor_right();
            }

            assert_eq!(buf.cursor, buf.user_buffer.len());

            // move left back to start
            while buf.cursor > 0 {
                assert!(buf.user_buffer.is_char_boundary(buf.cursor));
                buf.cursor_left();
            }

            assert_eq!(buf.cursor, 0);
        }

        #[test]
        fn cursor_right_then_left_is_identity() {
            let mut buf = Buffer {
                user_buffer: "abçd".to_string(),
                suggestion_buffer: "".to_string(),
                cursor: 1,
            };

            let start = buf.cursor;
            buf.cursor_right();
            buf.cursor_left();

            assert_eq!(buf.cursor, start);
        }

        #[test]
        fn cursor_left_then_right_is_identity() {
            let mut buf = Buffer {
                user_buffer: "abçd".to_string(),
                suggestion_buffer: "".to_string(),
                cursor: "abçd".len(),
            };

            let start = buf.cursor;
            buf.cursor_left();
            buf.cursor_right();

            assert_eq!(buf.cursor, start);
        }
    }
}
