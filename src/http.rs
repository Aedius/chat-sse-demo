use serde::{Deserialize, Serialize};
use serde_json::Result as SerdeResult;

#[derive(Serialize, Deserialize, Debug)]
pub struct ChatMess {
    name: String,
    mess: String,
}

pub fn decode_chat_mess(row: &str) -> SerdeResult<ChatMess> {
    let json = row.trim_end_matches("\u{0}");

    let m: ChatMess = serde_json::from_str(json)?;
    Ok(m)
}