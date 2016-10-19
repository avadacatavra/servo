/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::collections::HashMap;
use dom::bindings::str::{DOMString, USVString};
use dom::location::Location;
use dom::window::Window;

#[dom_struct]
pub struct CrossOrigin {
    propertyMap: HashMap<(USVString, USVString, u64),USVString>;   //key: (currentOrigin, objOrigin, propertyKey), value: propery descriptors
}

pub struct CrossOriginProperty {
    name: String,
    needsGet: Option<bool>,
    needsSet: Option<bool>,
}

impl CrossOriginProperty {
    pub fn new(propertyName: String, get: Option<bool>, set: Option<bool>) -> CrossOriginProperty {
        CrossOriginProperty {
            name: propertyName,
            needsGet: get,
            needsSet: set
        }
    }
}

trait CrossOriginProperties {
    fn crossOriginProperties(&self) -> HashMap ();
}

impl CrossOrigin {
    pub fn isPlatformObjectSameOrigin(){}

    pub fn crossOriginGetOwnPropertyHelper(){}

    pub fn crossOriginGet(){}

    pub fn crossOriginSet(){}

    pub fn crossOriginOwnPropertyKeys(){}
}

impl CrossOriginProperties for Location -> HashMap {    //FIXME this isn't a hashmap anymore-- go to bed
    [CrossOriginProperty::new("href", false, true), CrossOriginProperty::new("replace", None, None)] 
}

impl CrossOriginProperties for Window {

}

