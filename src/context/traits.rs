pub trait Context {
    fn to_context_string(&self) -> String{
        "".to_string() 
    }
}
pub trait  LLMforamt{
    fn to_json_format() -> String{
        "{}".to_string()
    }
}