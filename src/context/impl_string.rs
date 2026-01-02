use super::traits::Context;

impl Context for String {
    fn to_context_string(&self) -> String {
        self.clone()
    }
}
impl Context for &str {
    fn to_context_string(&self) -> String {
        self.to_string()
    }
}