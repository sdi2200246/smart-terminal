use serde::Deserialize;

#[derive(Deserialize , Debug)]
pub enum ProtocolError{
    Empty,
    NoTool,
}