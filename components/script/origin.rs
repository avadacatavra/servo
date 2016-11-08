/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use std::sync::Arc;
use url::{Host, Url};
use url::Origin as UrlOrigin;

/// A representation of an [origin](https://html.spec.whatwg.org/multipage/#origin-2).
#[derive(HeapSizeOf, JSTraceable, Eq, PartialEq, Hash, Debug, Clone)]
pub struct Origin {
    #[ignore_heap_size_of = "Arc<T> has unclear ownership semantics"]
    inner: Arc<UrlOrigin>,
}

impl Origin {
    /// Create a new origin comprising a unique, opaque identifier.
    pub fn opaque_identifier() -> Origin {
        Origin {
            inner: Arc::new(UrlOrigin::new_opaque()),
        }
    }

    /// Create a new origin for the given URL.
    pub fn new(url: &Url) -> Origin {
        Origin {
            inner: Arc::new(url.origin()),
        }
    }

    /// Does this origin represent a host/scheme/port tuple?
    pub fn is_scheme_host_port_tuple(&self) -> bool {
        self.inner.is_tuple()
    }

    /// Return the host associated with this origin.
    pub fn host(&self) -> Option<&Host<String>> {
        match *self.inner {
            UrlOrigin::Tuple(_, ref host, _) => Some(host),
            UrlOrigin::Opaque(..) => None,
        }
    }

    /// https://html.spec.whatwg.org/multipage/#same-origin
    pub fn same_origin(&self, other: &Origin) -> bool {
        self.inner == other.inner
    }
        //https://html.spec.whatwg.org/multipage/browsers.html#same-origin-domain
    pub fn same_origin_domain(&self, other: &Origin) -> bool {
        match *self.inner {
            UrlOrigin::Opaque(_) => self.inner == other.inner,
            UrlOrigin::Tuple(ref scheme, ref host, _) => {
                let b = match *other.inner {
                    UrlOrigin::Tuple(ref other_scheme, ref other_host, _) => {
                        println!("{} == {}", scheme, other_scheme);
                        println!("{} == {}", host, other_host);

                        scheme == other_scheme && host == other_host
                    },
                    _ => false,
                };
                b
            },
        }
    }

    pub fn copy(&self) -> Origin {
        Origin {
            inner: Arc::new((*self.inner).clone()),
        }
    }

    pub fn alias(&self) -> Origin {
        Origin {
            inner: self.inner.clone(),
        }
    }
}
