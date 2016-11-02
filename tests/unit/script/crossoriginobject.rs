use script::dom::crossoriginobject::CrossOrigin as XOW;
use script::origin::Origin;
use url::Url;

#[test]
fn is_platform_object_same_origin() {
	let a = XOW::new(&Origin::new(&Url::parse("http://example.com").unwrap()));
	let b = Origin::new(&Url::parse("http://example.com").unwrap());
	let c = Origin::new(&Url::parse("https://example.com").unwrap());

	assert!(a.isPlatformObjectSameOrigin(&b));
	assert!(!a.isPlatformObjectSameOrigin(&c));

}