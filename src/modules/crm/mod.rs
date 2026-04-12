// CRM Module
// Company, contact, and activity management

pub mod types;
pub mod companies;
pub mod contacts;
pub mod activities;

#[allow(unused_imports)]
pub use types::*;
pub use companies::*;
pub use contacts::*;
pub use activities::*;
