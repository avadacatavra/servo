/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use cssparser::RGBA;
use dom::bindings::codegen::Bindings::HTMLTableSectionElementBinding::{self, HTMLTableSectionElementMethods};
use dom::bindings::codegen::Bindings::NodeBinding::NodeMethods;
use dom::bindings::error::{ErrorResult, Fallible};
use dom::bindings::inheritance::Castable;
use dom::bindings::root::{DomRoot, LayoutDom, RootedReference};
use dom::bindings::str::DOMString;
use dom::document::Document;
use dom::element::{Element, RawLayoutElementHelpers};
use dom::htmlcollection::{CollectionFilter, HTMLCollection};
use dom::htmlelement::HTMLElement;
use dom::htmltablerowelement::HTMLTableRowElement;
use dom::node::{Node, window_from_node};
use dom::virtualmethods::VirtualMethods;
use dom_struct::dom_struct;
use html5ever::{LocalName, Prefix};
use style::attr::AttrValue;

#[dom_struct]
pub struct HTMLTableSectionElement {
    htmlelement: HTMLElement,
}

impl HTMLTableSectionElement {
    fn new_inherited(local_name: LocalName, prefix: Option<Prefix>, document: &Document)
                     -> HTMLTableSectionElement {
        HTMLTableSectionElement {
            htmlelement: HTMLElement::new_inherited(local_name, prefix, document),
        }
    }

    #[allow(unrooted_must_root)]
    pub fn new(local_name: LocalName, prefix: Option<Prefix>, document: &Document)
               -> DomRoot<HTMLTableSectionElement> {
        Node::reflect_node(Box::new(HTMLTableSectionElement::new_inherited(local_name, prefix, document)),
                           document,
                           HTMLTableSectionElementBinding::Wrap)
    }
}

#[derive(JSTraceable)]
struct RowsFilter;
impl CollectionFilter for RowsFilter {
    fn filter(&self, elem: &Element, root: &Node) -> bool {
        elem.is::<HTMLTableRowElement>() &&
            elem.upcast::<Node>().GetParentNode().r() == Some(root)
    }
}

impl HTMLTableSectionElementMethods for HTMLTableSectionElement {
    // https://html.spec.whatwg.org/multipage/#dom-tbody-rows
    fn Rows(&self) -> DomRoot<HTMLCollection> {
        HTMLCollection::create(&window_from_node(self), self.upcast(), Box::new(RowsFilter))
    }

    // https://html.spec.whatwg.org/multipage/#dom-tbody-insertrow
    fn InsertRow(&self, index: i32) -> Fallible<DomRoot<HTMLElement>> {
        let node = self.upcast::<Node>();
        insert_cell_or_row(
            &node,
            index,
            || self.Rows(),
            || HTMLTableRowElement::new(local_name!("tr"), None, &node.owner_doc()))
    }

    // https://html.spec.whatwg.org/multipage/#dom-tbody-deleterow
    fn DeleteRow(&self, index: i32) -> ErrorResult {
        let node = self.upcast::<Node>();
        delete_cell_or_row(
            &node,
            index,
            || self.Rows(),
            |n| n.is::<HTMLTableRowElement>())
    }
}

pub trait HTMLTableSectionElementLayoutHelpers {
    fn get_background_color(&self) -> Option<RGBA>;
}

#[allow(unsafe_code)]
impl HTMLTableSectionElementLayoutHelpers for LayoutDom<HTMLTableSectionElement> {
    fn get_background_color(&self) -> Option<RGBA> {
        unsafe {
            (&*self.upcast::<Element>().unsafe_get())
                .get_attr_for_layout(&ns!(), &local_name!("bgcolor"))
                .and_then(AttrValue::as_color)
                .cloned()
        }
    }
}

impl VirtualMethods for HTMLTableSectionElement {
    fn super_type(&self) -> Option<&VirtualMethods> {
        Some(self.upcast::<HTMLElement>() as &VirtualMethods)
    }

    fn parse_plain_attribute(&self, local_name: &LocalName, value: DOMString) -> AttrValue {
        match *local_name {
            local_name!("bgcolor") => AttrValue::from_legacy_color(value.into()),
            _ => self.super_type().unwrap().parse_plain_attribute(local_name, value),
        }
    }
}

/// Used by `HTMLTableSectionElement::InsertRow` and `HTMLTableRowElement::InsertCell`
    pub fn insert_cell_or_row<F, G, I>(node: &Node, index: i32, get_items: F, new_child: G) -> Fallible<DomRoot<HTMLElement>>
        where F: Fn() -> DomRoot<HTMLCollection>,
              G: Fn() -> DomRoot<I>,
              I: DerivedFrom<Node> + DerivedFrom<HTMLElement> + DomObject,
    {
        if index < -1 {
            return Err(Error::IndexSize);
        }

        let tr = new_child();


        {
            let tr_node = tr.upcast::<Node>();
            if index == -1 {
                node.InsertBefore(tr_node, None)?;
            } else {
                let items = get_items();
                let node = match items.elements_iter()
                                      .map(DomRoot::upcast::<Node>)
                                      .map(Some)
                                      .chain(iter::once(None))
                                      .nth(index as usize) {
                    None => return Err(Error::IndexSize),
                    Some(node) => node,
                };
                node.InsertBefore(tr_node, node.r())?;
            }
        }

        Ok(DomRoot::upcast::<HTMLElement>(tr))
    }

    /// Used by `HTMLTableSectionElement::DeleteRow` and `HTMLTableRowElement::DeleteCell`
    pub fn delete_cell_or_row<F, G>(node: &Node, index: i32, get_items: F, is_delete_type: G) -> ErrorResult
        where F: Fn() -> DomRoot<HTMLCollection>,
              G: Fn(&Element) -> bool
    {
        let element = match index {
            index if index < -1 => return Err(Error::IndexSize),
            -1 => {
                let last_child = node.upcast::<Node>().GetLastChild();
                match last_child.and_then(|n| n.inclusively_preceding_siblings()
                                                     .filter_map(DomRoot::downcast::<Element>)
                                                     .filter(|elem| is_delete_type(elem))
                                                     .next()) {
                    Some(element) => element,
                    None => return Ok(()),
                }
            },
            index => match get_items().Item(index as u32) {
                Some(element) => element,
                None => return Err(Error::IndexSize),
            },
        };

        element.upcast::<Node>().remove_self();
        Ok(())
    }
