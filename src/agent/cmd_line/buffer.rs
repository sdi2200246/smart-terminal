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

    pub fn push(&mut self , c:&str){
        self.user_buffer.push_str(c);
    }

    pub fn pop(&mut self){
        self.user_buffer.pop();
    }
}


impl Default for Buffer{

    fn default()->Self{
        Self{
            user_buffer:String::new(),
            suggestion_buffer:String::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn buffer_suffix() {
        let buffer = Buffer{
            user_buffer:"git".to_string(),
            suggestion_buffer:"git_push".to_string(),
        };
        assert_eq!(buffer.buffer_suffix() , "_push");

        let buffer = Buffer{
            user_buffer:"git".to_string(),
            suggestion_buffer:"piss".to_string(),
        };
        assert_eq!(buffer.buffer_suffix() , "");


        let buffer = Buffer{
            user_buffer:"".to_string(),
            suggestion_buffer:"piss".to_string(),
        };
        assert_eq!(buffer.buffer_suffix() , "piss");
    }
    #[test]
    fn buffer_merge_suggestion(){
        let mut buffer = Buffer{
            user_buffer:"me".to_string(),
            suggestion_buffer:"mepiss".to_string(),
        };
        buffer.merge_suggestion();
        assert_eq!(buffer.get_user_buffer() , "mepiss");
    }
    #[test]
    fn buffer_push() {
        let mut buf = Buffer::default();

        buf.push("g");
        buf.push("it");
        buf.push(" status");

        assert_eq!(buf.get_user_buffer(), "git status");
    }
}