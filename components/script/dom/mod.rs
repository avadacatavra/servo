/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! The implementation of the DOM.
//!
//! The DOM is comprised of interfaces (defined by specifications using
//! [WebIDL](https://heycam.github.io/webidl/)) that are implemented as Rust
//! structs in submodules of this module. Its implementation is documented
//! below.
//!
//! A DOM object and its reflector
//! ==============================
//!
//! The implementation of an interface `Foo` in Servo's DOM involves two
//! related but distinct objects:
//!
//! * the **DOM object**: an instance of the Rust struct `dom::foo::Foo`
//!   (marked with the `#[dom_struct]` attribute) on the Rust heap;
//! * the **reflector**: a `JSObject` allocated by SpiderMonkey, that owns the
//!   DOM object.
//!
//! Memory management
//! =================
//!
//! Reflectors of DOM objects, and thus the DOM objects themselves, are managed
//! by the SpiderMonkey Garbage Collector. Thus, keeping alive a DOM object
//! is done through its reflector.
//!
//! For more information, see:
//!
//! * rooting pointers on the stack:
//!   the [`Root`](bindings/root/struct.Root.html) smart pointer;
//! * tracing pointers in member fields: the [`Dom`](bindings/root/struct.Dom.html),
//!   [`MutNullableDom`](bindings/root/struct.MutNullableDom.html) and
//!   [`MutDom`](bindings/root/struct.MutDom.html) smart pointers and
//!   [the tracing implementation](bindings/trace/index.html);
//! * rooting pointers from across thread boundaries or in channels: the
//!   [`Trusted`](bindings/refcounted/struct.Trusted.html) smart pointer;
//!
//! Inheritance
//! ===========
//!
//! Rust does not support struct inheritance, as would be used for the
//! object-oriented DOM APIs. To work around this issue, Servo stores an
//! instance of the superclass in the first field of its subclasses. (Note that
//! it is stored by value, rather than in a smart pointer such as `Dom<T>`.)
//!
//! This implies that a pointer to an object can safely be cast to a pointer
//! to all its classes.
//!
//! This invariant is enforced by the lint in
//! `plugins::lints::inheritance_integrity`.
//!
//! Interfaces which either derive from or are derived by other interfaces
//! implement the `Castable` trait, which provides three methods `is::<T>()`,
//! `downcast::<T>()` and `upcast::<T>()` to cast across the type hierarchy
//! and check whether a given instance is of a given type.
//!
//! ```ignore
//! use dom::bindings::inheritance::Castable;
//! use dom::element::Element;
//! use dom::htmlelement::HTMLElement;
//! use dom::htmlinputelement::HTMLInputElement;
//!
//! if let Some(elem) = node.downcast::<Element> {
//!     if elem.is::<HTMLInputElement>() {
//!         return elem.upcast::<HTMLElement>();
//!     }
//! }
//! ```
//!
//! Furthermore, when discriminating a given instance against multiple
//! interface types, code generation provides a convenient TypeId enum
//! which can be used to write `match` expressions instead of multiple
//! calls to `Castable::is::<T>`. The `type_id()` method of an instance is
//! provided by the farthest interface it derives from, e.g. `EventTarget`
//! for `HTMLMediaElement`. For convenience, that method is also provided
//! on the `Node` interface to avoid unnecessary upcasts to `EventTarget`.
//!
//! ```ignore
//! use dom::bindings::inheritance::{EventTargetTypeId, NodeTypeId};
//!
//! match *node.type_id() {
//!     EventTargetTypeId::Node(NodeTypeId::CharacterData(_)) => ...,
//!     EventTargetTypeId::Node(NodeTypeId::Element(_)) => ...,
//!     ...,
//! }
//! ```
//!
//! Construction
//! ============
//!
//! DOM objects of type `T` in Servo have two constructors:
//!
//! * a `T::new_inherited` static method that returns a plain `T`, and
//! * a `T::new` static method that returns `DomRoot<T>`.
//!
//! (The result of either method can be wrapped in `Result`, if that is
//! appropriate for the type in question.)
//!
//! The latter calls the former, boxes the result, and creates a reflector
//! corresponding to it by calling `dom::bindings::utils::reflect_dom_object`
//! (which yields ownership of the object to the SpiderMonkey Garbage Collector).
//! This is the API to use when creating a DOM object.
//!
//! The former should only be called by the latter, and by subclasses'
//! `new_inherited` methods.
//!
//! DOM object constructors in JavaScript correspond to a `T::Constructor`
//! static method. This method is always fallible.
//!
//! Destruction
//! ===========
//!
//! When the SpiderMonkey Garbage Collector discovers that the reflector of a
//! DOM object is garbage, it calls the reflector's finalization hook. This
//! function deletes the reflector's DOM object, calling its destructor in the
//! process.
//!
//! Mutability and aliasing
//! =======================
//!
//! Reflectors are JavaScript objects, and as such can be freely aliased. As
//! Rust does not allow mutable aliasing, mutable borrows of DOM objects are
//! not allowed. In particular, any mutable fields use `Cell` or `DomRefCell`
//! to manage their mutability.
//!
//! `Reflector` and `DomObject`
//! =============================
//!
//! Every DOM object has a `Reflector` as its first (transitive) member field.
//! This contains a `*mut JSObject` that points to its reflector.
//!
//! The `FooBinding::Wrap` function creates the reflector, stores a pointer to
//! the DOM object in the reflector, and initializes the pointer to the reflector
//! in the `Reflector` field.
//!
//! The `DomObject` trait provides a `reflector()` method that returns the
//! DOM object's `Reflector`. It is implemented automatically for DOM structs
//! through the `#[dom_struct]` attribute.
//!
//! Implementing methods for a DOM object
//! =====================================
//!
//! * `dom::bindings::codegen::Bindings::FooBindings::FooMethods` for methods
//!   defined through IDL;
//! * `&self` public methods for public helpers;
//! * `&self` methods for private helpers.
//!
//! Accessing fields of a DOM object
//! ================================
//!
//! All fields of DOM objects are private; accessing them from outside their
//! module is done through explicit getter or setter methods.
//!
//! Inheritance and casting
//! =======================
//!
//! All DOM interfaces part of an inheritance chain (i.e. interfaces
//! that derive others or are derived from) implement the trait `Castable`
//! which provides both downcast and upcasts.
//!
//! ```ignore
//! # use script::dom::bindings::inheritance::Castable;
//! # use script::dom::element::Element;
//! # use script::dom::node::Node;
//! # use script::dom::htmlelement::HTMLElement;
//! fn f(element: &Element) {
//!     let base = element.upcast::<Node>();
//!     let derived = element.downcast::<HTMLElement>().unwrap();
//! }
//! ```
//!
//! Adding a new DOM interface
//! ==========================
//!
//! Adding a new interface `Foo` requires at least the following:
//!
//! * adding the new IDL file at `components/script/dom/webidls/Foo.webidl`;
//! * creating `components/script/dom/foo.rs`;
//! * listing `foo.rs` in `components/script/dom/mod.rs`;
//! * defining the DOM struct `Foo` with a `#[dom_struct]` attribute, a
//!   superclass or `Reflector` member, and other members as appropriate;
//! * implementing the
//!   `dom::bindings::codegen::Bindings::FooBindings::FooMethods` trait for
//!   `Foo`;
//! * adding/updating the match arm in create_element in
//!   `components/script/dom/create.rs` (only applicable to new types inheriting
//!   from `HTMLElement`)
//!
//! More information is available in the [bindings module](bindings/index.html).
//!
//! Accessing DOM objects from layout
//! =================================
//!
//! Layout code can access the DOM through the
//! [`LayoutDom`](bindings/root/struct.LayoutDom.html) smart pointer. This does not
//! keep the DOM object alive; we ensure that no DOM code (Garbage Collection
//! in particular) runs while the layout thread is accessing the DOM.
//!
//! Methods accessible to layout are implemented on `LayoutDom<Foo>` using
//! `LayoutFooHelpers` traits.

#[macro_use]
pub mod macros;

pub mod types {
    #[cfg(not(target_env = "msvc"))]
    include!(concat!(env!("OUT_DIR"), "/InterfaceTypes.rs"));
    #[cfg(target_env = "msvc")]
    include!(concat!(env!("OUT_DIR"), "/build/InterfaceTypes.rs"));
}

#[cfg(feature = "servo")] pub mod abstractworker;
#[cfg(feature = "servo")] pub mod abstractworkerglobalscope;
#[cfg(feature = "servo")] pub mod activation;
pub mod attr;
pub mod beforeunloadevent;
pub mod bindings;
#[cfg(feature = "servo")] pub mod blob;
#[cfg(feature = "servo")] pub mod bluetooth;
#[cfg(feature = "servo")] pub mod bluetoothadvertisingevent;
#[cfg(feature = "servo")] pub mod bluetoothcharacteristicproperties;
#[cfg(feature = "servo")] pub mod bluetoothdevice;
#[cfg(feature = "servo")] pub mod bluetoothpermissionresult;
#[cfg(feature = "servo")] pub mod bluetoothremotegattcharacteristic;
#[cfg(feature = "servo")] pub mod bluetoothremotegattdescriptor;
#[cfg(feature = "servo")] pub mod bluetoothremotegattserver;
#[cfg(feature = "servo")] pub mod bluetoothremotegattservice;
#[cfg(feature = "servo")] pub mod bluetoothuuid;
#[cfg(feature = "servo")] pub mod canvasgradient;
#[cfg(feature = "servo")] pub mod canvaspattern;
#[cfg(feature = "servo")] pub mod canvasrenderingcontext2d;
pub mod characterdata;
#[cfg(feature = "servo")] pub mod client;
#[cfg(feature = "servo")] pub mod closeevent;
pub mod comment;
#[cfg(feature = "servo")] pub mod compositionevent;
#[cfg(feature = "servo")] pub mod console;
mod create;
#[cfg(feature = "servo")] pub mod crypto;
#[cfg(feature = "servo")] pub mod css;
#[cfg(feature = "servo")] pub mod cssconditionrule;
#[cfg(feature = "servo")] pub mod cssfontfacerule;
#[cfg(feature = "servo")] pub mod cssgroupingrule;
#[cfg(feature = "servo")] pub mod cssimportrule;
#[cfg(feature = "servo")] pub mod csskeyframerule;
#[cfg(feature = "servo")] pub mod csskeyframesrule;
#[cfg(feature = "servo")] pub mod cssmediarule;
#[cfg(feature = "servo")] pub mod cssnamespacerule;
pub mod cssrule;
pub mod cssrulelist;
pub mod cssstyledeclaration;
#[cfg(feature = "servo")] pub mod cssstylerule;
pub mod cssstylesheet;
#[cfg(feature = "servo")] pub mod cssstylevalue;
#[cfg(feature = "servo")] pub mod csssupportsrule;
#[cfg(feature = "servo")] pub mod cssviewportrule;
#[cfg(feature = "servo")] pub mod customelementregistry;
#[cfg(feature = "servo")] pub mod customevent;
#[cfg(feature = "servo")] pub mod dedicatedworkerglobalscope;
pub mod dissimilaroriginlocation;
pub mod dissimilaroriginwindow;
pub mod document;
pub mod documentfragment;
pub mod documenttype;
pub mod domexception;
#[cfg(feature = "servo")] pub mod domimplementation;
#[cfg(feature = "servo")] pub mod dommatrix;
#[cfg(feature = "servo")] pub mod dommatrixreadonly;
#[cfg(feature = "servo")] pub mod domparser;
#[cfg(feature = "servo")] pub mod dompoint;
#[cfg(feature = "servo")] pub mod dompointreadonly;
#[cfg(feature = "servo")] pub mod domquad;
#[cfg(feature = "servo")] pub mod domrect;
#[cfg(feature = "servo")] pub mod domrectreadonly;
#[cfg(feature = "servo")] pub mod domstringmap;
#[cfg(feature = "servo")] pub mod domtokenlist;
pub mod element;
pub mod errorevent;
pub mod event;
#[cfg(feature = "servo")] pub mod eventsource;
pub mod eventtarget;
#[cfg(feature = "servo")] pub mod extendableevent;
#[cfg(feature = "servo")] pub mod extendablemessageevent;
#[cfg(feature = "servo")] pub mod file;
#[cfg(feature = "servo")] pub mod filelist;
#[cfg(feature = "servo")] pub mod filereader;
#[cfg(feature = "servo")] pub mod filereadersync;
#[cfg(feature = "servo")] pub mod focusevent;
#[cfg(feature = "servo")] pub mod forcetouchevent;
#[cfg(feature = "servo")] pub mod formdata;
#[cfg(feature = "servo")] pub mod gamepad;
#[cfg(feature = "servo")] pub mod gamepadbutton;
#[cfg(feature = "servo")] pub mod gamepadbuttonlist;
#[cfg(feature = "servo")] pub mod gamepadevent;
#[cfg(feature = "servo")] pub mod gamepadlist;
pub mod globalscope;
#[cfg(feature = "servo")] pub mod hashchangeevent;
#[cfg(feature = "servo")] pub mod headers;
#[cfg(feature = "servo")] pub mod history;
#[cfg(feature = "servo")] pub mod htmlanchorelement;
#[cfg(feature = "servo")] pub mod htmlareaelement;
#[cfg(feature = "servo")] pub mod htmlaudioelement;
#[cfg(feature = "servo")] pub mod htmlbaseelement;
#[cfg(feature = "servo")] pub mod htmlbodyelement;
#[cfg(feature = "servo")] pub mod htmlbrelement;
#[cfg(feature = "servo")] pub mod htmlbuttonelement;
#[cfg(feature = "servo")] pub mod htmlcanvaselement;
pub mod htmlcollection;
#[cfg(feature = "servo")] pub mod htmldataelement;
#[cfg(feature = "servo")] pub mod htmldatalistelement;
#[cfg(feature = "servo")] pub mod htmldetailselement;
#[cfg(feature = "servo")] pub mod htmldialogelement;
#[cfg(feature = "servo")] pub mod htmldirectoryelement;
#[cfg(feature = "servo")] pub mod htmldivelement;
#[cfg(feature = "servo")] pub mod htmldlistelement;
#[cfg(feature = "servo")] pub mod htmlelement;
#[cfg(feature = "servo")] pub mod htmlembedelement;
#[cfg(feature = "servo")] pub mod htmlfieldsetelement;
#[cfg(feature = "servo")] pub mod htmlfontelement;
#[cfg(feature = "servo")] pub mod htmlformcontrolscollection;
#[cfg(feature = "servo")] pub mod htmlformelement;
#[cfg(feature = "servo")] pub mod htmlframeelement;
#[cfg(feature = "servo")] pub mod htmlframesetelement;
#[cfg(feature = "servo")] pub mod htmlheadelement;
#[cfg(feature = "servo")] pub mod htmlheadingelement;
#[cfg(feature = "servo")] pub mod htmlhrelement;
#[cfg(feature = "servo")] pub mod htmlhtmlelement;
#[cfg(feature = "servo")] pub mod htmliframeelement;
#[cfg(feature = "servo")] pub mod htmlimageelement;
#[cfg(feature = "servo")] pub mod htmlinputelement;
#[cfg(feature = "servo")] pub mod htmllabelelement;
#[cfg(feature = "servo")] pub mod htmllegendelement;
#[cfg(feature = "servo")] pub mod htmllielement;
#[cfg(feature = "servo")] pub mod htmllinkelement;
#[cfg(feature = "servo")] pub mod htmlmapelement;
#[cfg(feature = "servo")] pub mod htmlmediaelement;
#[cfg(feature = "servo")] pub mod htmlmetaelement;
#[cfg(feature = "servo")] pub mod htmlmeterelement;
#[cfg(feature = "servo")] pub mod htmlmodelement;
#[cfg(feature = "servo")] pub mod htmlobjectelement;
#[cfg(feature = "servo")] pub mod htmlolistelement;
#[cfg(feature = "servo")] pub mod htmloptgroupelement;
#[cfg(feature = "servo")] pub mod htmloptionelement;
#[cfg(feature = "servo")] pub mod htmloptionscollection;
#[cfg(feature = "servo")] pub mod htmloutputelement;
#[cfg(feature = "servo")] pub mod htmlparagraphelement;
#[cfg(feature = "servo")] pub mod htmlparamelement;
#[cfg(feature = "servo")] pub mod htmlpreelement;
#[cfg(feature = "servo")] pub mod htmlprogresselement;
#[cfg(feature = "servo")] pub mod htmlquoteelement;
#[cfg(feature = "servo")] pub mod htmlscriptelement;
#[cfg(feature = "servo")] pub mod htmlselectelement;
#[cfg(feature = "servo")] pub mod htmlsourceelement;
#[cfg(feature = "servo")] pub mod htmlspanelement;
#[cfg(feature = "servo")] pub mod htmlstyleelement;
#[cfg(feature = "servo")] pub mod htmltablecaptionelement;
#[cfg(feature = "servo")] pub mod htmltablecellelement;
#[cfg(feature = "servo")] pub mod htmltablecolelement;
#[cfg(feature = "servo")] pub mod htmltabledatacellelement;
#[cfg(feature = "servo")] pub mod htmltableelement;
#[cfg(feature = "servo")] pub mod htmltableheadercellelement;
#[cfg(feature = "servo")] pub mod htmltablerowelement;
#[cfg(feature = "servo")] pub mod htmltablesectionelement;
#[cfg(feature = "servo")] pub mod htmltemplateelement;
#[cfg(feature = "servo")] pub mod htmltextareaelement;
#[cfg(feature = "servo")] pub mod htmltimeelement;
#[cfg(feature = "servo")] pub mod htmltitleelement;
#[cfg(feature = "servo")] pub mod htmltrackelement;
#[cfg(feature = "servo")] pub mod htmlulistelement;
#[cfg(feature = "servo")] pub mod htmlunknownelement;
#[cfg(feature = "servo")] pub mod htmlvideoelement;
#[cfg(feature = "servo")] pub mod imagedata;
#[cfg(feature = "servo")] pub mod inputevent;
#[cfg(feature = "servo")] pub mod keyboardevent;
pub mod location;
#[cfg(feature = "servo")] pub mod mediaerror;
#[cfg(feature = "servo")] pub mod medialist;
#[cfg(feature = "servo")] pub mod mediaquerylist;
#[cfg(feature = "servo")] pub mod mediaquerylistevent;
#[cfg(feature = "servo")] pub mod messageevent;
#[cfg(feature = "servo")] pub mod mimetype;
#[cfg(feature = "servo")] pub mod mimetypearray;
#[cfg(feature = "servo")] pub mod mouseevent;
pub mod mutationobserver;
pub mod mutationrecord;
pub mod namednodemap;
#[cfg(feature = "servo")] pub mod navigator;
#[cfg(feature = "servo")] pub mod navigatorinfo;
pub mod node;
pub mod nodeiterator;
pub mod nodelist;
#[cfg(feature = "servo")] pub mod pagetransitionevent;
#[cfg(feature = "servo")] pub mod paintrenderingcontext2d;
#[cfg(feature = "servo")] pub mod paintsize;
#[cfg(feature = "servo")] pub mod paintworkletglobalscope;
#[cfg(feature = "servo")] pub mod performance;
#[cfg(feature = "servo")] pub mod performanceentry;
#[cfg(feature = "servo")] pub mod performancemark;
#[cfg(feature = "servo")] pub mod performancemeasure;
#[cfg(feature = "servo")] pub mod performanceobserver;
#[cfg(feature = "servo")] pub mod performanceobserverentrylist;
#[cfg(feature = "servo")] pub mod performancepainttiming;
#[cfg(feature = "servo")] pub mod performancetiming;
pub mod permissions;
pub mod permissionstatus;
#[cfg(feature = "servo")] pub mod plugin;
#[cfg(feature = "servo")] pub mod pluginarray;
#[cfg(feature = "servo")] pub mod popstateevent;
pub mod processinginstruction;
#[cfg(feature = "servo")] pub mod progressevent;
pub mod promise;
pub mod promisenativehandler;
#[cfg(feature = "servo")] pub mod radionodelist;
pub mod range;
#[cfg(feature = "servo")] pub mod request;
#[cfg(feature = "servo")] pub mod response;
#[cfg(feature = "servo")] pub mod screen;
#[cfg(feature = "servo")] pub mod serviceworker;
#[cfg(feature = "servo")] pub mod serviceworkercontainer;
#[cfg(feature = "servo")] pub mod serviceworkerglobalscope;
#[cfg(feature = "servo")] pub mod serviceworkerregistration;
pub mod servoparser;
#[cfg(feature = "servo")] pub mod storage;
#[cfg(feature = "servo")] pub mod storageevent;
#[cfg(feature = "servo")] pub mod stylepropertymapreadonly;
pub mod stylesheet;
pub mod stylesheetlist;
#[cfg(feature = "servo")] pub mod svgelement;
#[cfg(feature = "servo")] pub mod svggraphicselement;
#[cfg(feature = "servo")] pub mod svgsvgelement;
#[cfg(feature = "servo")] pub mod testbinding;
#[cfg(feature = "servo")] pub mod testbindingiterable;
#[cfg(feature = "servo")] pub mod testbindingpairiterable;
#[cfg(feature = "servo")] pub mod testbindingproxy;
#[cfg(feature = "servo")] pub mod testrunner;
#[cfg(feature = "servo")] pub mod testworklet;
#[cfg(feature = "servo")] pub mod testworkletglobalscope;
pub mod text;
#[cfg(feature = "servo")] pub mod textcontrol;
#[cfg(feature = "servo")] pub mod textdecoder;
#[cfg(feature = "servo")] pub mod textencoder;
#[cfg(feature = "servo")] pub mod touch;
#[cfg(feature = "servo")] pub mod touchevent;
#[cfg(feature = "servo")] pub mod touchlist;
#[cfg(feature = "servo")] pub mod transitionevent;
pub mod treewalker;
#[cfg(feature = "servo")] pub mod uievent;
#[cfg(feature = "servo")] pub mod url;
pub mod urlhelper;
#[cfg(feature = "servo")] pub mod urlsearchparams;
#[cfg(feature = "servo")] pub mod userscripts;
#[cfg(feature = "servo")] pub mod validation;
#[cfg(feature = "servo")] pub mod validitystate;
#[cfg(feature = "servo")] pub mod values;
pub mod virtualmethods;
#[cfg(feature = "servo")] pub mod vr;
#[cfg(feature = "servo")] pub mod vrdisplay;
#[cfg(feature = "servo")] pub mod vrdisplaycapabilities;
#[cfg(feature = "servo")] pub mod vrdisplayevent;
#[cfg(feature = "servo")] pub mod vreyeparameters;
#[cfg(feature = "servo")] pub mod vrfieldofview;
#[cfg(feature = "servo")] pub mod vrframedata;
#[cfg(feature = "servo")] pub mod vrpose;
#[cfg(feature = "servo")] pub mod vrstageparameters;
#[cfg(feature = "servo")] pub mod webgl_extensions;
#[cfg(feature = "servo")] pub use self::webgl_extensions::ext::*;
#[cfg(feature = "servo")] pub mod webgl2renderingcontext;
#[cfg(feature = "servo")] pub mod webgl_validations;
#[cfg(feature = "servo")] pub mod webglactiveinfo;
#[cfg(feature = "servo")] pub mod webglbuffer;
#[cfg(feature = "servo")] pub mod webglcontextevent;
#[cfg(feature = "servo")] pub mod webglframebuffer;
#[cfg(feature = "servo")] pub mod webglobject;
#[cfg(feature = "servo")] pub mod webglprogram;
#[cfg(feature = "servo")] pub mod webglrenderbuffer;
#[cfg(feature = "servo")] pub mod webglrenderingcontext;
#[cfg(feature = "servo")] pub mod webglshader;
#[cfg(feature = "servo")] pub mod webglshaderprecisionformat;
#[cfg(feature = "servo")] pub mod webgltexture;
#[cfg(feature = "servo")] pub mod webgluniformlocation;
#[cfg(feature = "servo")] pub mod websocket;
pub mod window;
pub mod windowproxy;
#[cfg(feature = "servo")] pub mod worker;
pub mod workerglobalscope;
pub mod workerlocation;
pub mod workernavigator;
#[cfg(feature = "servo")] pub mod worklet;
#[cfg(feature = "servo")] pub mod workletglobalscope;
#[cfg(feature = "servo")] pub mod xmldocument;
#[cfg(feature = "servo")] pub mod xmlhttprequest;
#[cfg(feature = "servo")] pub mod xmlhttprequesteventtarget;
#[cfg(feature = "servo")] pub mod xmlhttprequestupload;
