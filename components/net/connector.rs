/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use antidote::Mutex;
use hyper;
use hyper::Result;
use hyper::client::Pool;
use hyper::net::{HttpStream, HttpsConnector, SslClient};
use hyper_openssl;
use openssl;
use openssl::ssl::{SSL_OP_NO_COMPRESSION, SSL_OP_NO_SSLV2, SSL_OP_NO_SSLV3, SSL_VERIFY_PEER};
use openssl::ssl::{Ssl, SslContext, SslContextBuilder, SslMethod};
use openssl::x509::X509StoreContextRef;
use rustls;
use rustls::RootCertStore;
use servo_config::resource_files::resources_dir_path;       //FIXME are we using this or the cert file arg
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use time;

pub type Connector = HttpsConnector<ServoSslClient>;

// The basic logic here is to prefer ciphers with ECDSA certificates, Forward
// Secrecy, AES GCM ciphers, AES ciphers, and finally 3DES ciphers.
// A complete discussion of the issues involved in TLS configuration can be found here:
// https://wiki.mozilla.org/Security/Server_Side_TLS
const DEFAULT_CIPHERS: &'static str = concat!(
    "ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256:",
    "ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-GCM-SHA384:",
    "DHE-RSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES128-SHA256:",
    "ECDHE-RSA-AES128-SHA256:ECDHE-ECDSA-AES256-SHA384:ECDHE-RSA-AES256-SHA384:",
    "ECDHE-ECDSA-AES128-SHA:ECDHE-RSA-AES128-SHA:ECDHE-ECDSA-AES256-SHA:",
    "ECDHE-RSA-AES256-SHA:DHE-RSA-AES128-SHA256:DHE-RSA-AES128-SHA:",
    "DHE-RSA-AES256-SHA256:DHE-RSA-AES256-SHA:ECDHE-RSA-DES-CBC3-SHA:",
    "ECDHE-ECDSA-DES-CBC3-SHA:AES128-GCM-SHA256:AES256-GCM-SHA384:",
    "AES128-SHA256:AES256-SHA256:AES128-SHA:AES256-SHA"
);

pub fn create_http_connector(certificate_file: &str) -> Arc<Pool<Connector>> {
    let mut context = SslContextBuilder::new(SslMethod::tls()).unwrap();
    context.set_ca_file(certificate_file);
    context.set_cipher_list(DEFAULT_CIPHERS).unwrap();
    context.set_options(SSL_OP_NO_SSLV2 | SSL_OP_NO_SSLV3 | SSL_OP_NO_COMPRESSION);

    //create the rustls root cert store
    let ca_pem = File::open(certificate_file).unwrap();
    let mut ca_pem = BufReader::new(ca_pem);
    let mut root_store = RootCertStore::empty();
    root_store.add_pem_file(&mut ca_pem).unwrap().0;

    let servo_connector = ServoSslConnector {
        context: Arc::new(context.build()),
        roots: Arc::new(root_store),
    };

    let connector = HttpsConnector::new(ServoSslClient {
        connector: Arc::new(servo_connector),
    });

    Arc::new(Pool::with_connector(Default::default(), connector))
}

#[derive(Clone)]
pub struct ServoSslClient {
    connector: Arc<ServoSslConnector>,
}

impl SslClient for ServoSslClient {
    type Stream = hyper_openssl::SslStream<HttpStream>;

    fn wrap_client(&self, stream: HttpStream, host: &str) -> Result<Self::Stream> {
        let start = time::precise_time_ns();
        let r = match self.connector.connect(host, stream) {
            Ok(stream) => Ok(hyper_openssl::SslStream(Arc::new(Mutex::new(stream)))),
            Err(err) => Err(err),
        };
        let end = time::precise_time_ns();
        info!("openssl verify time: {} ns", end-start);
        r
    }
}

#[derive(Clone)]
pub struct ServoSslConnector {
    context: Arc<SslContext>,
    roots: Arc<RootCertStore>,
}

impl ServoSslConnector {
    pub fn connect(&self, domain: &str, stream: HttpStream) -> Result<openssl::ssl::SslStream<HttpStream>>
    {
        let mut ssl = Ssl::new(&self.context).unwrap();
        ssl.set_hostname(domain).unwrap();
        let domain = domain.to_owned();
        let roots = self.roots.clone();

        ssl.set_verify_callback(SSL_VERIFY_PEER, move |p, x| {
            openssl_verify_fn(&domain, p, x)
            //rustls_verify(&domain, &roots, p, x)
        });



        match ssl.connect(stream) {
            Ok(stream) => Ok(stream),
            Err(err) => Err(hyper::Error::Ssl(Box::new(err))),
        }
    }
}

// for profiling purposes
fn openssl_verify_fn(domain: &str, preverify_ok: bool, x509_ctx: &X509StoreContextRef) -> bool {
    verify::verify_callback(&domain, preverify_ok, x509_ctx)
}

//TODO figure out what to do with preverify_ok
fn rustls_verify(domain: &str,
                roots: &RootCertStore,
                preverify_ok: bool,
                x509_ctx: &X509StoreContextRef) -> bool {
    // create presented certs
    let mut presented_certs = vec!();
    match x509_ctx.chain() {
        Some(chain) => {
            for cert in chain {
                presented_certs.push(rustls::Certificate(cert.to_der().unwrap()));
            }
        },
        None => (),
    };

    // verify certificate
    //this is where we can measure 
    match rustls::verify_server_cert(&roots, &presented_certs, &domain) {
        Ok(_) => true,
        Err(error) => { error!("Verification error: {:?}", error);
                      false },
    }
}

//for testing purposes only
mod verify {
    use std::net::IpAddr;
    use std::str;

    use openssl::nid;
    use openssl::x509::{X509StoreContextRef, X509Ref, X509NameRef, GeneralName};
    use openssl::stack::Stack;

    pub fn verify_callback(domain: &str,
                           preverify_ok: bool,
                           x509_ctx: &X509StoreContextRef)
                           -> bool {
        if !preverify_ok || x509_ctx.error_depth() != 0 {
            return preverify_ok;
        }

        match x509_ctx.current_cert() {
            Some(x509) => verify_hostname(domain, &x509),
            None => true,
        }
    }

    fn verify_hostname(domain: &str, cert: &X509Ref) -> bool {
        match cert.subject_alt_names() {
            Some(names) => verify_subject_alt_names(domain, names),
            None => verify_subject_name(domain, &cert.subject_name()),
        }
    }

    fn verify_subject_alt_names(domain: &str, names: Stack<GeneralName>) -> bool {
        let ip = domain.parse();

        for name in &names {
            match ip {
                Ok(ip) => {
                    if let Some(actual) = name.ipaddress() {
                        if matches_ip(&ip, actual) {
                            return true;
                        }
                    }
                }
                Err(_) => {
                    if let Some(pattern) = name.dnsname() {
                        if matches_dns(pattern, domain, false) {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    fn verify_subject_name(domain: &str, subject_name: &X509NameRef) -> bool {
        if let Some(pattern) = subject_name.entries_by_nid(nid::COMMONNAME).next() {
            let pattern = match str::from_utf8(pattern.data().as_slice()) {
                Ok(pattern) => pattern,
                Err(_) => return false,
            };

            // Unlike with SANs, IP addresses in the subject name don't have a
            // different encoding. We need to pass this down to matches_dns to
            // disallow wildcard matches with bogus patterns like *.0.0.1
            let is_ip = domain.parse::<IpAddr>().is_ok();

            if matches_dns(&pattern, domain, is_ip) {
                return true;
            }
        }

        false
    }

    fn matches_dns(mut pattern: &str, mut hostname: &str, is_ip: bool) -> bool {
        // first strip trailing . off of pattern and hostname to normalize
        if pattern.ends_with('.') {
            pattern = &pattern[..pattern.len() - 1];
        }
        if hostname.ends_with('.') {
            hostname = &hostname[..hostname.len() - 1];
        }

        matches_wildcard(pattern, hostname, is_ip).unwrap_or_else(|| pattern == hostname)
    }

    fn matches_wildcard(pattern: &str, hostname: &str, is_ip: bool) -> Option<bool> {
        // IP addresses and internationalized domains can't involved in wildcards
        if is_ip || pattern.starts_with("xn--") {
            return None;
        }

        let wildcard_location = match pattern.find('*') {
            Some(l) => l,
            None => return None,
        };

        let mut dot_idxs = pattern.match_indices('.').map(|(l, _)| l);
        let wildcard_end = match dot_idxs.next() {
            Some(l) => l,
            None => return None,
        };

        // Never match wildcards if the pattern has less than 2 '.'s (no *.com)
        //
        // This is a bit dubious, as it doesn't disallow other TLDs like *.co.uk.
        // Chrome has a black- and white-list for this, but Firefox (via NSS) does
        // the same thing we do here.
        //
        // The Public Suffix (https://www.publicsuffix.org/) list could
        // potentially be used here, but it's both huge and updated frequently
        // enough that management would be a PITA.
        if dot_idxs.next().is_none() {
            return None;
        }

        // Wildcards can only be in the first component
        if wildcard_location > wildcard_end {
            return None;
        }

        let hostname_label_end = match hostname.find('.') {
            Some(l) => l,
            None => return None,
        };

        // check that the non-wildcard parts are identical
        if pattern[wildcard_end..] != hostname[hostname_label_end..] {
            return Some(false);
        }

        let wildcard_prefix = &pattern[..wildcard_location];
        let wildcard_suffix = &pattern[wildcard_location + 1..wildcard_end];

        let hostname_label = &hostname[..hostname_label_end];

        // check the prefix of the first label
        if !hostname_label.starts_with(wildcard_prefix) {
            return Some(false);
        }

        // and the suffix
        if !hostname_label[wildcard_prefix.len()..].ends_with(wildcard_suffix) {
            return Some(false);
        }

        Some(true)
    }

    fn matches_ip(expected: &IpAddr, actual: &[u8]) -> bool {
        match (expected, actual.len()) {
            (&IpAddr::V4(ref addr), 4) => actual == addr.octets(),
            (&IpAddr::V6(ref addr), 16) => {
                let segments = [((actual[0] as u16) << 8) | actual[1] as u16,
                                ((actual[2] as u16) << 8) | actual[3] as u16,
                                ((actual[4] as u16) << 8) | actual[5] as u16,
                                ((actual[6] as u16) << 8) | actual[7] as u16,
                                ((actual[8] as u16) << 8) | actual[9] as u16,
                                ((actual[10] as u16) << 8) | actual[11] as u16,
                                ((actual[12] as u16) << 8) | actual[13] as u16,
                                ((actual[14] as u16) << 8) | actual[15] as u16];
                segments == addr.segments()
            }
            _ => false,
        }
    }
}
