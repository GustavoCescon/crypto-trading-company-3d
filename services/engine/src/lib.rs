pub mod db;
pub mod market;
pub mod trading;
pub type Error = Box<dyn std::error::Error + Send + Sync>;
