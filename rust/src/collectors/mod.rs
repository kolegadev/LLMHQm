//! Collectors module - data ingestion layer

pub mod binance;
pub mod wired;

pub use binance::{BinanceCollector, LiquidationEvent, spawn_collector};
pub use wired::{spawn_wired_collector, DataRouter};
