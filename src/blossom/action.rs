use std::str::FromStr;

#[derive(PartialEq, Clone)]
pub enum Action {
    Upload,
    Has,
    Get,
    List,
    Delete,
}

impl FromStr for Action {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "upload" => Ok(Self::Upload),
            "has" => Ok(Self::Has),
            "get" => Ok(Self::Get),
            "list" => Ok(Self::List),
            "delete" => Ok(Self::Delete),
            _ => Err("invalid enum variant".into()),
        }
    }
}
