/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! Callback interfaces are traits specified in UDL which can be implemented by foreign languages.
//!
//! # Using callback interfaces
//!
//! 1. Define a Rust trait.
//!
//! This toy example defines a way of Rust accessing a key-value store exposed
//! by the host operating system (e.g. the key chain).
//!
//! ```
//! trait Keychain: Send {
//!   fn get(&self, key: String) -> Option<String>;
//!   fn put(&self, key: String, value: String);
//! }
//! ```
//!
//! 2. Define a callback interface in the UDL
//!
//! ```idl
//! callback interface Keychain {
//!     string? get(string key);
//!     void put(string key, string data);
//! };
//! ```
//!
//! 3. And allow it to be passed into Rust.
//!
//! Here, we define a constructor to pass the keychain to rust, and then another method
//! which may use it.
//!
//! In UDL:
//! ```idl
//! object Authenticator {
//!     constructor(Keychain keychain);
//!     void login();
//! }
//! ```
//!
//! In Rust:
//!
//! ```
//!# trait Keychain: Send {
//!#  fn get(&self, key: String) -> Option<String>;
//!#  fn put(&self, key: String, value: String);
//!# }
//! struct Authenticator {
//!   keychain: Box<dyn Keychain>,
//! }
//!
//! impl Authenticator {
//!   pub fn new(keychain: Box<dyn Keychain>) -> Self {
//!     Self { keychain }
//!   }
//!   pub fn login(&self) {
//!     let username = self.keychain.get("username".into());
//!     let password = self.keychain.get("password".into());
//!   }
//! }
//! ```
//! 4. Create an foreign language implementation of the callback interface.
//!
//! In this example, here's a Kotlin implementation.
//!
//! ```kotlin
//! class AndroidKeychain: Keychain {
//!     override fun get(key: String): String? {
//!         // … elide the implementation.
//!         return value
//!     }
//!     override fun put(key: String) {
//!         // … elide the implementation.
//!     }
//! }
//! ```
//! 5. Pass the implementation to Rust.
//!
//! Again, in Kotlin
//!
//! ```kotlin
//! val authenticator = Authenticator(AndroidKeychain())
//! authenticator.login()
//! ```
//!
//! # How it works.
//!
//! ## High level
//!
//! Uniffi generates a protocol or interface in client code in the foreign language must implement.
//!
//! For each callback interface, a `CallbackInternals` (on the Foreign Language side) and `ForeignCallbackInternals`
//! (on Rust side) manages the process through a `ForeignCallback`. There is one `ForeignCallback` per callback interface.
//!
//! Passing a callback interface implementation from foreign language (e.g. `AndroidKeychain`) into Rust causes the
//! `KeychainCallbackInternals` to store the instance in a handlemap.
//!
//! The object handle is passed over to Rust, and used to instantiate a struct `KeychainProxy` which implements
//! the trait. This proxy implementation is generate by Uniffi. The `KeychainProxy` object is then passed to
//! client code as `Box<dyn Keychain>`.
//!
//! Methods on `KeychainProxy` objects (e.g. `self.keychain.get("username".into())`) encode the arguments into a `RustBuffer`.
//! Using the `ForeignCallback`, it calls the `CallbackInternals` object on the foreign language side using the
//! object handle, and the method selector.
//!
//! The `CallbackInternals` object unpacks the arguments from the passed buffer, gets the object out from the handlemap,
//! and calls the actual implementation of the method.
//!
//! If there's a return value, it is packed up in to another `RustBuffer` and used as the return value for
//! `ForeignCallback`. The caller of `ForeignCallback`, the `KeychainProxy` unpacks the returned buffer into the correct
//! type and then returns to client code.
//!

use super::RustBuffer;
use std::sync::atomic::{AtomicUsize, Ordering};

/// ForeignCallback is the Rust representation of a foreign language function.
/// It is the basis for all callbacks interfaces. It is registered exactly once per callback interface,
/// at library start up time.
/// Calling this method is only done by generated objects which mirror callback interfaces objects in the foreign language.
/// The `handle` is the key into a handle map on the other side of the FFI used to look up the foreign language object
/// that implements the callback interface/trait.
/// The `method` selector specifies the method that will be called on the object, by looking it up in a list of methods from
/// the IDL. The index is 1 indexed. Note that the list of methods is generated by at uniffi from the IDL and used in all
/// bindings: so we can rely on the method list being stable within the same run of uniffi.
pub type ForeignCallback =
    unsafe extern "C" fn(handle: u64, method: u32, args: RustBuffer) -> RustBuffer;

/// The method index used by the Drop trait to communicate to the foreign language side that Rust has finished with it,
/// and it can be deleted from the handle map.
pub const IDX_CALLBACK_FREE: u32 = 0;

// Overly-paranoid sanity checking to ensure that these types are
// convertible between each-other. `transmute` actually should check this for
// us too, but this helps document the invariants we rely on in this code.
//
// Note that these are guaranteed by
// https://rust-lang.github.io/unsafe-code-guidelines/layout/function-pointers.html
// and thus this is a little paranoid.
static_assertions::assert_eq_size!(usize, ForeignCallback);
static_assertions::assert_eq_size!(usize, Option<ForeignCallback>);

/// Struct to hold a foreign callback.
pub struct ForeignCallbackInternals {
    callback_ptr: AtomicUsize,
}

const EMPTY_PTR: usize = 0;

impl ForeignCallbackInternals {
    pub const fn new() -> Self {
        ForeignCallbackInternals {
            callback_ptr: AtomicUsize::new(EMPTY_PTR),
        }
    }

    pub fn set_callback(&self, callback: ForeignCallback) {
        let as_usize = callback as usize;
        let old_ptr = self.callback_ptr.compare_exchange(
            EMPTY_PTR,
            as_usize,
            Ordering::SeqCst,
            Ordering::SeqCst,
        );
        match old_ptr {
            // We get the previous value back. If this is anything except EMPTY_PTR,
            // then this has been set before we get here.
            Ok(EMPTY_PTR) => (),
            _ =>
            // This is an internal bug, the other side of the FFI should ensure
            // it sets this only once.
            {
                panic!("Bug: call set_callback multiple times. This is likely a uniffi bug")
            }
        };
    }

    pub fn get_callback(&self) -> Option<ForeignCallback> {
        let ptr_value = self.callback_ptr.load(Ordering::SeqCst);
        unsafe { std::mem::transmute::<usize, Option<ForeignCallback>>(ptr_value) }
    }
}
