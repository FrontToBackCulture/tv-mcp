// Email campaign sending — uses AWS SES directly from the desktop app.
// Tracking endpoints (open pixel, click redirect, unsubscribe, bounce webhook)
// remain on tv-api since they need public URLs.

pub mod send;
pub mod campaigns;

pub use send::*;
