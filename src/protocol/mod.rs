pub mod event;
pub mod adapter;
pub mod adapter_manager;
pub mod matcher;
pub mod traits;
pub mod gb28181;
pub mod onvif;
pub mod rtsp;
pub mod websocket;

pub use event::ProtocolType;
pub use adapter::{AdapterRegistry, SignalAdapter};
pub use adapter_manager::{get_adapter, set_adapter, remove_adapter, clear_adapters, cleanup_expired_subscriptions_all, AdapterEntry};
pub use matcher::ProtocolMatcher;