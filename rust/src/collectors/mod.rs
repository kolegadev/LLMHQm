//! Collectors module - data ingestion layer

pub mod binance;

pub use binance::{BinanceCollector, LiquidationEvent, spawn_collector};
