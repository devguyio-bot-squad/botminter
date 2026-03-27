pub mod bridge_adapter;
mod event_watcher;
mod heartbeat;
pub mod inbox;
mod multiplexer;
mod prompt_template;
mod queue;
mod types;

pub use event_watcher::{EventWatcher, EventWatcherConfig, EventWatcherError};
pub use heartbeat::{
    Heartbeat, HeartbeatConfig, HeartbeatError, HeartbeatPending, HeartbeatShutdown,
};
pub use multiplexer::{
    Multiplexer, MultiplexerConfig, MultiplexerError, MultiplexerInput, MultiplexerOutput,
    MultiplexerShutdown,
};
pub use prompt_template::{
    parse_github_repo, read_member_name, read_member_role, render_brain_prompt,
    surface_brain_prompt, BrainPromptVars,
};
pub use queue::PromptQueue;
pub use types::{BrainMessage, BridgeOutput, Priority};
