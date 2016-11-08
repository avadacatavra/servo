/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::collections::HashMap;
use dom::bindings::str::{DOMString, USVString};
use heapsize::HeapSizeOf;
use dom::bindings::trace::JSTraceable;
use origin::{Origin};
use url::Url;
use js::jsapi::JSObject;

//#[dom_struct]
#[cfg_attr(feature = "servo", derive(HeapSizeOf))]
#[derive(JSTraceable)]
pub struct CrossOrigin {
    propertyMap: HashMap<CrossOriginKey, PropertyDescriptor>,   //key: (currentOrigin, objOrigin, propertyKey), value: propery descriptors
    origin: Origin,
}

#[derive(PartialEq, Eq, Hash, JSTraceable, Debug, Clone)]
struct CrossOriginKey {
    curr_origin: Origin,
    obj_origin: Origin,
    prop_key: String,
}

pub struct CrossOriginProperty {    //TODO maybe make this an enum
    name: String,                   //FIXME String or &str?
    needsGet: Option<bool>,         //FIXME do these need to be options or can i just assume true/false if None
    needsSet: Option<bool>,
}

impl PartialEq for CrossOriginProperty {
    fn eq(&self, other: &CrossOriginProperty) -> bool {
        self.name == other.name
    }
}

#[cfg_attr(feature = "servo", derive(HeapSizeOf))]
#[derive(JSTraceable)]
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

pub trait CrossOriginProperties {
    fn get_properties(&self) -> Vec<CrossOriginProperty>; 
}

impl CrossOrigin {
    pub fn new(origin: &Origin) -> CrossOrigin{
        CrossOrigin {propertyMap: HashMap::new(), origin: origin.copy() }
    }

    //TODO needs to take a platform obj not Origin
    pub fn isPlatformObjectSameOrigin(&self, obj: &Origin ) -> bool {
        self.origin.same_origin_domain(obj)
    }

    pub fn crossOriginGetOwnPropertyHelper(&self, 
                                           property_name: String) 
                                           -> Option<PropertyDescriptor> {
       None 
    }

    pub fn crossOriginGet(&self,
                          property_name: String,
                          receiver: Option<JSObject>)   //TODO
                          -> Option<PropertyDescriptor> {
        None
    }

    pub fn crossOriginSet(&self,
                          property_name: String,
                          receiver: Option<JSObject>)   //TODO
                          -> bool {
        false
    }

    pub fn crossOriginOwnPropertyKeys(&mut self) -> Vec<String> {    //TODO check for rust-> js list
        let map_len = self.propertyMap.len();
        let mut key_list = Vec::with_capacity(map_len);
        for (ref key, _) in self.propertyMap.iter_mut() {
            key_list.push(key.prop_key.clone());
        }
        key_list
    }
}

//FIXME default behavior for testing
impl CrossOriginProperties for CrossOrigin {
    fn get_properties(&self) -> Vec<CrossOriginProperty> {
        vec!(CrossOriginProperty::new("href".to_string(), Some(false), Some(true)), CrossOriginProperty::new("replace".to_string(), None, None))
    }
}

impl HeapSizeOf for CrossOrigin {
    fn heap_size_of_children(&self) -> usize {
        0   //FIXME
    }
}

/*impl CrossOriginProperties for Location {
    fn get_properties(&self)-> Vec<CrossOriginProperty> {  
        //pass in an object instead? do window and location share a superclass? ...a trait should do it...
        vec!(CrossOriginProperty::new("href".to_string(), Some(false), Some(true)), CrossOriginProperty::new("replace".to_string(), None, None)) 
    }
}*/

/*impl CrossOriginProperties for Window {
    fn get_properties(&self) -> Vec<CrossOriginProperty> {
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

    //TODO repeat for each e that is an element of O's document-tree child browsing contest name
    //property set. Add {[[Property]], e} as the last element of crossOriginProperties and return
    }
}*/

