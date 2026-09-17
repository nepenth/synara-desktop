//! Experimental widget host — Core owner plus isolated Tauri webview.

#![allow(dead_code)]
#![allow(unused_imports)]

pub use synara_core::app::widgets::*;

pub mod host;
pub mod live;

pub use live::start as start_widget_owner;
