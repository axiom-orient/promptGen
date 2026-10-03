//! UTF-8 byte and count limits of the shared external contract.
pub const MAXIMUM_MODEL_MESSAGES_PER_TURN: usize = 128;
pub const MAXIMUM_MODEL_MESSAGE_BYTES: usize = 128 * 1024;
pub const MAXIMUM_MODEL_MESSAGE_SCALARS: usize = 128 * 1024;
pub const MAXIMUM_MODEL_TOOLS_PER_TURN: usize = 64;
pub const MAXIMUM_TOOL_CALLS_PER_TURN: usize = 64;
pub const MAXIMUM_TOOL_CALL_ID_BYTES: usize = 192;
pub const MAXIMUM_TOOL_NAME_BYTES: usize = 160;
pub const MAXIMUM_TOOL_ARGUMENT_BYTES: usize = 256 * 1024;
pub const MAXIMUM_TOOL_ARGUMENTS_BYTES: usize = 1024 * 1024;
pub const MAXIMUM_MODEL_RESPONSE_BYTES: usize = 8 * 1024 * 1024;
pub const MAXIMUM_IMAGE_BYTES: usize = 8 * 1024 * 1024;
pub const MAXIMUM_PROVIDER_REQUEST_BYTES: usize = 64 * 1024 * 1024;
pub const MAXIMUM_MODEL_FRAME_BYTES: usize = MAXIMUM_PROVIDER_REQUEST_BYTES;
pub const MAXIMUM_IMAGE_FRAME_BYTES: usize = 16 * 1024 * 1024;
pub const MAXIMUM_PROVIDER_TIMEOUT_MS: u64 = 30 * 60 * 1_000;
