/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::collections::HashMap;
use dom::bindings::str::{DOMString, USVString};
use dom::location::Location;
use dom::window::Window;

pub struct CrossOrigin {
    propertyMap: HashMap<(String, String, String), PropertyDescriptor>   //key: (currentOrigin, objOrigin, propertyKey), value: propery descriptors
}

pub struct CrossOriginProperty {    //TODO maybe make this an enum
    name: String,                   //FIXME String or &str?
    needsGet: Option<bool>,         //FIXME do these need to be options or can i just assume true/false if None
    needsSet: Option<bool>,
}

pub struct PropertyDescriptor {
    value: String,
    writeable: bool,
    enumerable: bool,
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
    fn new(&self) -> Vec<CrossOriginProperty>;
}

impl CrossOrigin {
    pub fn new(&self) -> CrossOrigin{
        self.propertyMap = HashMap::new();
    }


    pub fn isPlatformObjectSameOrigin(){}

    pub fn crossOriginGetOwnPropertyHelper(){}

    pub fn crossOriginGet(){}

    pub fn crossOriginSet(){}

    pub fn crossOriginOwnPropertyKeys(){}
}

impl CrossOriginProperties for Location {
    fn new(&self)-> Vec<CrossOriginProperty> {  //pass in an object instead? do window and location share a superclass? ...a trait should do it...
        vec!(CrossOriginProperty::new("href".to_string(), Some(false), Some(true)), CrossOriginProperty::new("replace".to_string(), None, None)) 
    }
}

impl CrossOriginProperties for Window {
    fn new(&self) -> Vec<CrossOriginProperty> {
        vec!(CrossOriginProperty::new("window".to_string(), Some(true), Some(false)),
         CrossOriginProperty::new("self".to_string(), Some(true), Some(false)),
         CrossOriginProperty::new("location".to_string(), Some(true), Some(true)),
         CrossOriginProperty::new("close".to_string(), None, None),
         CrossOriginProperty::new("closed".to_string(), Some(true), Some(false)),
         CrossOriginProperty::new("focus".to_string(), None, None),
         CrossOriginProperty::new("blur".to_string(), None, None),
         CrossOriginProperty::new("frames".to_string(), Some(true), Some(false)),
         CrossOriginProperty::new("length".to_string(), Some(true), Some(false)),
         CrossOriginProperty::new("top".to_string(), Some(true), Some(false)),
         CrossOriginProperty::new("opener".to_string(), Some(true), Some(false)),
         CrossOriginProperty::new("parent".to_string(), Some(true), Some(false)),
         CrossOriginProperty::new("postMessage".to_string(), None, None))

    //repeat for each e that is an element of O's document-tree child browsing contest name
    //property set. Add {[[Property]], e} as the last element of crossOriginProperties and return
    }
}

