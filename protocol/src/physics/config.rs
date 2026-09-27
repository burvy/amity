/// How far (metres) a predicted position may be from the server's before the
/// client rolls back and resimulates
pub const ROLLBACK_POSITION: f32 = 0.01;
/// How far (metres per second) a predicted velocity may be from the server's
/// before the client rolls back and resimulates
pub const ROLLBACK_VELOCITY: f32 = 0.01;
