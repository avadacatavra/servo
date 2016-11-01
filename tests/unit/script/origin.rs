/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use script::origin::Origin;
use url::Url;

#[test]
fn same_origin() {
    let a = Origin::new(&Url::parse("http://example.com/a.html").unwrap());
    let b = Origin::new(&Url::parse("http://example.com/b.html").unwrap());
    assert!(a.same_origin(&b));
    assert_eq!(a.is_scheme_host_port_tuple(), true);
}

#[test]
fn identical_origin() {
    let a = Origin::new(&Url::parse("http://example.com/a.html").unwrap());
    assert!(a.same_origin(&a));
}

#[test]
fn cross_origin() {
    let a = Origin::new(&Url::parse("http://example.com/a.html").unwrap());
    let b = Origin::new(&Url::parse("http://example.org/b.html").unwrap());
    assert!(!a.same_origin(&b));
}

#[test]
fn opaque_same_origin_domain() {
    let a = Origin::opaque_identifier();
    let b = a.alias();
    println!("{:?}, {:?}", a, b);
    assert!(a.same_origin_domain(&b));
}

#[test]
fn same_origin_domain() {
    let mut a = Origin::new(&Url::parse("http://example.com").unwrap());
    let mut b = Origin::new(&Url::parse("http://example.com").unwrap());
    println!("{:?}, {:?}", a, b);
    assert!(a.same_origin_domain(&b));

    a = Origin::new(&Url::parse("http://example.com:412").unwrap());
    b = Origin::new(&Url::parse("http://example.com:317").unwrap());
    assert!(a.same_origin_domain(&b));

    //TODO same_origin_domain is false when a.domain == null and b.domain == example.org
    /*a = Origin::new(&Url::parse("http://example.com").unwrap());    //domain null
    b = Origin::new(&Url::parse("http://example.com").unwrap());    //domain example.org
    assert_eq!(a.same_origin_domain(&b), false);*/

    a = Origin::new(&Url::parse("https://example.com").unwrap());
    b = Origin::new(&Url::parse("http://example.com").unwrap());
    assert_eq!(a.same_origin_domain(&b), false);

}



#[test]
fn alias_same_origin() {
    let a = Origin::new(&Url::parse("http://example.com/a.html").unwrap());
    let b = Origin::new(&Url::parse("http://example.com/b.html").unwrap());
    let c = b.alias();
    assert!(a.same_origin(&c));
    assert!(b.same_origin(&b));
    assert!(c.same_origin(&b));
    assert_eq!(c.is_scheme_host_port_tuple(), true);
}

#[test]
fn alias_cross_origin() {
    let a = Origin::new(&Url::parse("http://example.com/a.html").unwrap());
    let b = Origin::new(&Url::parse("http://example.org/b.html").unwrap());
    let c = b.alias();
    assert!(!a.same_origin(&c));
    assert!(b.same_origin(&c));
    assert!(c.same_origin(&c));
}

#[test]
fn opaque() {
    let a = Origin::opaque_identifier();
    let b = Origin::opaque_identifier();
    assert!(!a.same_origin(&b));
    assert_eq!(a.is_scheme_host_port_tuple(), false);
}

#[test]
fn opaque_clone() {
    let a = Origin::opaque_identifier();
    let b = a.alias();
    assert!(a.same_origin(&b));
    assert_eq!(a.is_scheme_host_port_tuple(), false);
}
