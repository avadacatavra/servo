/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

#![cfg_attr(feature = "unstable", feature(core_intrinsics))]
#![cfg_attr(feature = "unstable", feature(on_unimplemented))]
#![feature(ascii_ctype)]
#![feature(conservative_impl_trait)]
#![feature(const_fn)]
#![feature(mpsc_select)]
#![feature(plugin)]
#![feature(proc_macro)]
#![feature(splice)]
#![feature(string_retain)]

#![deny(unsafe_code)]
#![allow(non_snake_case)]

#![doc = "The script crate contains all matters DOM."]

#![plugin(script_plugins)]
#![cfg_attr(not(feature = "unrooted_must_root_lint"), allow(unknown_lints))]

#[cfg(feature = "servo")] extern crate angle;
extern crate app_units;
#[cfg(feature = "servo")] extern crate audio_video_metadata;
#[cfg(feature = "servo")] extern crate base64;
#[macro_use]
#[cfg(feature = "servo")] extern crate bitflags;
#[cfg(feature = "servo")] extern crate bluetooth_traits;
#[cfg(feature = "servo")] extern crate byteorder;
#[cfg(feature = "servo")] extern crate canvas_traits;
#[cfg(feature = "servo")] extern crate caseless;
extern crate chrono;
#[cfg(feature = "servo")] extern crate cookie as cookie_rs;
#[macro_use] extern crate cssparser;
#[macro_use] extern crate deny_public_fields;
#[cfg(feature = "servo")]  extern crate devtools_traits;
extern crate dom_struct;
#[macro_use]
#[cfg(feature = "servo")]  extern crate domobject_derive;
extern crate encoding_rs;
extern crate euclid;
extern crate fnv;
#[cfg(feature = "servo")] extern crate gleam;
#[cfg(feature = "servo")] extern crate half;
#[macro_use] extern crate html5ever;
#[cfg(feature = "servo")] #[macro_use]
#[cfg(feature = "servo")] extern crate hyper;
#[cfg(feature = "servo")] extern crate hyper_serde;
#[cfg(feature = "servo")] extern crate image;
#[cfg(feature = "servo")] extern crate ipc_channel;
#[macro_use]
#[cfg(feature = "servo")]  extern crate jstraceable_derive;
#[macro_use]
#[cfg(feature = "servo")] extern crate lazy_static;
extern crate libc;
#[macro_use]
#[cfg(feature = "servo")] extern crate log;
#[macro_use] extern crate malloc_size_of;
#[macro_use] extern crate malloc_size_of_derive;
#[cfg(feature = "servo")] extern crate metrics;
#[macro_use]
#[cfg(feature = "servo")] extern crate mime;
#[cfg(feature = "servo")] extern crate mime_guess;
extern crate mitochondria;
#[macro_use]
extern crate mozjs as js;
extern crate msg;
#[cfg(feature = "servo")] extern crate net_traits;
extern crate num_traits;
#[cfg(feature = "servo")] extern crate offscreen_gl_context;
extern crate open;
#[cfg(feature = "servo")] extern crate parking_lot;
#[cfg(feature = "servo")] extern crate phf;
#[macro_use]
#[cfg(feature = "servo")] extern crate profile_traits;
#[cfg(feature = "servo")] extern crate ref_filter_map;
extern crate ref_slice;
#[cfg(feature = "servo")] extern crate regex;
extern crate script_layout_interface;
extern crate script_traits;
extern crate selectors;
#[cfg(feature = "servo")] extern crate serde;
#[cfg(feature = "servo")] extern crate servo_allocator;
extern crate servo_arc;
#[macro_use] extern crate servo_atoms;
extern crate servo_config;
#[cfg(feature = "servo")] extern crate servo_geometry;
#[cfg(feature = "servo")] extern crate servo_rand;
extern crate servo_url;
extern crate smallvec;
#[macro_use]
extern crate style;
extern crate style_traits;
#[cfg(feature = "servo")] extern crate swapper;
extern crate time;
#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
extern crate tinyfiledialogs;
#[cfg(feature = "servo")] extern crate unicode_segmentation;
extern crate url;
#[cfg(feature = "servo")] extern crate utf8;
extern crate uuid;
#[cfg(feature = "servo")] extern crate webrender_api;
#[cfg(feature = "servo")] extern crate webvr_traits;
#[cfg(feature = "servo")] extern crate xml5ever;

#[macro_use]
mod task;

#[cfg(feature = "servo")] mod body;
#[cfg(feature = "servo")] pub mod clipboard_provider;
#[cfg(feature = "servo")] mod devtools;
pub mod document_loader;
#[macro_use]
mod dom;
#[cfg(feature = "servo")] pub mod fetch;
#[cfg(feature = "servo")] mod layout_image;
#[cfg(feature = "servo")] mod mem;
mod microtask;
#[cfg(feature = "servo")] mod network_listener;
#[cfg(feature = "servo")] pub mod script_runtime;
#[allow(unsafe_code)]
pub mod script_thread;
#[cfg(feature = "servo")] mod serviceworker_manager;
#[cfg(feature = "servo")] mod serviceworkerjob;
#[cfg(feature = "servo")] mod stylesheet_loader;
#[cfg(feature = "servo")] mod task_source;
#[cfg(feature = "servo")] pub mod test;
#[cfg(feature = "servo")] pub mod textinput;
#[cfg(feature = "servo")] mod timers;
#[cfg(feature = "servo")] mod unpremultiplytable;
#[cfg(feature = "servo")] mod webdriver_handlers;

/// A module with everything layout can use from script.
///
/// Try to keep this small!
///
/// TODO(emilio): A few of the FooHelpers can go away, presumably...
#[cfg(feature = "servo")]
pub mod layout_exports {
    pub use dom::bindings::inheritance::{CharacterDataTypeId, ElementTypeId};
    pub use dom::bindings::inheritance::{HTMLElementTypeId, NodeTypeId};
    pub use dom::bindings::root::LayoutDom;
    pub use dom::characterdata::LayoutCharacterDataHelpers;
    pub use dom::document::{Document, LayoutDocumentHelpers, PendingRestyle};
    pub use dom::element::{Element, LayoutElementHelpers, RawLayoutElementHelpers};
    pub use dom::node::NodeFlags;
    pub use dom::node::{LayoutNodeHelpers, Node};
    pub use dom::text::Text;
}

use dom::bindings::codegen::RegisterBindings;
use dom::bindings::proxyhandler;
use script_traits::SWManagerSenders;
use serviceworker_manager::ServiceWorkerManager;

#[cfg(target_os = "linux")]
#[allow(unsafe_code)]
fn perform_platform_specific_initialization() {
    use std::mem;
    // 4096 is default max on many linux systems
    const MAX_FILE_LIMIT: libc::rlim_t = 4096;

    // Bump up our number of file descriptors to save us from impending doom caused by an onslaught
    // of iframes.
    unsafe {
        let mut rlim: libc::rlimit = mem::uninitialized();
        match libc::getrlimit(libc::RLIMIT_NOFILE, &mut rlim) {
            0 => {
                if rlim.rlim_cur >= MAX_FILE_LIMIT {
                    // we have more than enough
                    return;
                }

                rlim.rlim_cur = match rlim.rlim_max {
                    libc::RLIM_INFINITY => MAX_FILE_LIMIT,
                    _ => {
                        if rlim.rlim_max < MAX_FILE_LIMIT {
                            rlim.rlim_max
                        } else {
                            MAX_FILE_LIMIT
                        }
                    }
                };
                match libc::setrlimit(libc::RLIMIT_NOFILE, &rlim) {
                    0 => (),
                    _ => warn!("Failed to set file count limit"),
                };
            },
            _ => warn!("Failed to get file count limit"),
        };
    }
}

#[cfg(not(target_os = "linux"))]
fn perform_platform_specific_initialization() {}

pub fn init_service_workers(sw_senders: SWManagerSenders) {
    // Spawn the service worker manager passing the constellation sender
    ServiceWorkerManager::spawn_manager(sw_senders);
}

#[allow(unsafe_code)]
pub fn init() {
    unsafe {
        proxyhandler::init();

        // Create the global vtables used by the (generated) DOM
        // bindings to implement JS proxies.
        RegisterBindings::RegisterProxyHandlers();
    }

    perform_platform_specific_initialization();
}
