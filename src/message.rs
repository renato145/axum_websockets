
pub enum WebsocketMessage {
    Ping(Vec<u8>),
    Close,
}
