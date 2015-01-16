pub static SCREEN_WIDTH: f32 = 800.;
pub static SCREEN_HEIGHT: f32 = 600.;

// 50 ms timesteps
pub const TIME_STEP: f32 = 0.05;
pub const MAX_FRAME_TIME: f32 = 0.250;

// 10s timeout
pub const CONN_TIMEOUT: u32 = 10000;
pub const PROTO_ID: u32 = 0xD05F1575;
pub const MAX_PACKET_SIZE: usize = 1400;
// 1s ping interval
pub const PING_INTERVAL: u32 = 1000;
