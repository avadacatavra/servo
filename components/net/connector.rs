/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use hyper::client::Pool;
use hyper::net::{HttpsConnector};
//use hyper_openssl::OpensslClient;       //just use hypers client not hyper_openssl
use hyper::net::{SslClient, SslServer, NetworkStream, HttpStream};
use openssl::x509::{X509StoreContextRef, X509, X509Ref};
use openssl::ssl::{Ssl, SslContextBuilder, SslContext, SslMethod, HandshakeError};
use hyper;
use openssl::ssl::SslStream as OpensslStream;
use hyper_openssl::SslStream;
use openssl;
use std::io::{Read, Write};

use openssl::ssl::{SSL_OP_NO_COMPRESSION, SSL_OP_NO_SSLV2, SSL_OP_NO_SSLV3, SSL_VERIFY_PEER};
use openssl::ssl::{SslConnector, SslConnectorBuilder};
use openssl::error::ErrorStack;
use std::sync::Arc;
use util::resource_files::resources_dir_path;
use std::fmt::Debug;
use antidote::Mutex;
//use std::result::Result;
use hyper::Result;

use rustls;

use untrusted;
use untrusted::Input;
use rustls::internal::pemfile;
use rustls::RootCertStore;
use std::io::{BufReader};
use std::fs::File;
use time;
use openssl::hash::MessageDigest;

use std::fmt::Write as plz;

pub type Connector = HttpsConnector<ServoSslClient>;
//pub struct HttpsConnector<S: SslClient, C: NetworkConnector = HttpConnector>

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

pub fn create_http_connector() -> Arc<Pool<Connector>> {
    let ca_file = &resources_dir_path()
        .expect("Need certificate file to make network requests")
        .join("certs");
    let mut context = SslContextBuilder::new(SslMethod::tls()).unwrap();
    context.set_ca_file(ca_file);
    context.set_cipher_list(DEFAULT_CIPHERS).unwrap();
    context.set_options(SSL_OP_NO_SSLV2 | SSL_OP_NO_SSLV3 | SSL_OP_NO_COMPRESSION);
    let servo_connector = ServoSslConnector { 
        context: Arc::new(context.build()) 
    };
    let connector = HttpsConnector::new(ServoSslClient {
        //context: Arc::new(context)
        connector: Arc::new(servo_connector)
    });
    Arc::new(Pool::with_connector(Default::default(), connector))
    
}

#[derive(Clone)]
pub struct ServoSslClient{
    //context: Arc<SslContextBuilder>,
    connector: Arc<ServoSslConnector>,
}

impl SslClient for ServoSslClient {
    type Stream = SslStream<HttpStream>;

    fn wrap_client(&self, stream: HttpStream, host: &str) -> Result<Self::Stream> {
        debug!("wrapping client");
        match self.connector.connect(host, stream){
            Ok(stream) => Ok(SslStream(Arc::new(Mutex::new(stream)))),
            Err(err) => Err(err)
        }
    }
}

#[derive(Clone)]
pub struct ServoSslConnector {
    context: Arc<SslContext>
}

impl ServoSslConnector {
    pub fn connect(&self, domain: &str, stream: HttpStream) -> Result<openssl::ssl::SslStream<HttpStream>>
        //where S: Read + Write + Sync + Send + Debug
    {
        let mut ssl = Ssl::new(&self.context).unwrap();
        ssl.set_hostname(domain).unwrap();
        let domain = domain.to_owned();
        ssl.set_verify_callback(SSL_VERIFY_PEER, move |p, x| {
            //::openssl_verify::verify_callback(&host, p, x)        //private module
            webpki_verify(&domain, p, x)
        });

        debug!("connecting");

        match ssl.connect(stream) {
            Ok(stream) => Ok(stream),
            Err(err) => Err(hyper::Error::Ssl(Box::new(err))),
        }
    }
}

/*
    Now the plan is to use the rustls representations, stick them in those, then use those to pass shit to webpki
    untrusted::Input borrows its backing buffer instead of owning it
     x509.to_der().unwrap() returns a Vec, which actually owns its contents.
 */
fn webpki_verify (domain: &str,
                preverify_ok: bool,
                x509_ctx: &X509StoreContextRef) -> bool {

    // create a rustls root store 
    let mut roots = RootCertStore::empty();
    let ca_file = &resources_dir_path()
            .expect("Need certificate file to make network requests")
            .join("certs");
    let ca_pem = File::open(ca_file).unwrap();
    let mut ca_pem = BufReader::new(ca_pem);
    let r = roots.add_pem_file(&mut ca_pem);
    debug!("Result of adding certs: {:?}", r);  // 153

    //get vector of webpki trustanchors FIXME not necessary
    //let trust_roots = roots.get_roots_as_trust_anchor();

    // get intermediate cert chain
    let chain = get_inter_vec(x509_ctx);
    debug!("chain length: {}", chain.len());

    let cert = match x509_ctx.current_cert() {
        Some(x509) => { let der = x509.to_der().unwrap();
                        debug!("cert extracted");
                        rustls::Certificate(der) },
        None => return false,
    };
    //let mut cert_chain = vec!(cert);
    let mut cert_chain = Vec::new();
    cert_chain.push(cert);
    for c in chain {
        cert_chain.push(rustls::Certificate(c.to_der().unwrap()));
    }

    debug!("presented certs: {}", cert_chain.len());

    //TODO let's take a closer look at the certs here

    //ok these certs look right, but they aren't being passed properly somehow
    for c in cert_chain.clone() {
        let mut bytes = String::new();
        for b in c.0 {
            write!(&mut bytes, "{:02X}", b);
        }

     
        debug!("certificate in connector: \n{}", bytes)
    }

    // there's an error in build_chain called from verify_is_valid_tls_cert

    match rustls::verify_server_cert(&roots,
                               &cert_chain,
                               domain) {
        Ok(_) => true,
        Err(err) => {debug!("{:?}", err); false}
    }

}

fn get_inter_vec(x509_ctx: &X509StoreContextRef) -> Vec<&X509Ref> {
    let mut inter_vec = vec!();
    match x509_ctx.chain() {
        //Some(chain) => vec!(),//.extend(chain),
        Some(chain) => inter_vec.extend(chain),
        None => (),
    };
    inter_vec
}
