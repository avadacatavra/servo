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
use webpki::*;
use std::fmt::Debug;
use antidote::Mutex;
//use std::result::Result;
use hyper::Result;

use untrusted::Input;
use webpki::trust_anchor_util;
use rustls::internal::pemfile;
use std::io::{BufReader};
use std::fs::File;
use time;
use openssl::hash::MessageDigest;

static ALL_SIGALGS: &'static [&'static SignatureAlgorithm] = &[
    &ECDSA_P256_SHA256,
    &ECDSA_P256_SHA384,
    &ECDSA_P384_SHA256,
    &ECDSA_P384_SHA384,
    &RSA_PKCS1_2048_8192_SHA1,
    &RSA_PKCS1_2048_8192_SHA256,
    &RSA_PKCS1_2048_8192_SHA384,
    &RSA_PKCS1_2048_8192_SHA512,
    &RSA_PKCS1_3072_8192_SHA384
];

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

fn rustls_verify (domain: &str,
                preverify_ok: bool,
                x509_ctx: &X509StoreContextRef) -> bool {
    true
}

// ok. let's try a new approach. instead of this, why don't you use the shit that rustls did
/*fn webpki_verify (domain: &str,
                  preverify_ok: bool,
                  x509_ctx: &X509StoreContextRef) -> bool {
    //not sure if i need  preverify_ok

    debug!("verifying");

    let ca_file = &resources_dir_path()
            .expect("Need certificate file to make network requests")
            .join("certs");
    //let ca_pem = include_bytes!("../../resources/certs");   //argument must be string literal
    let ca_pem = File::open(ca_file).unwrap();
    let mut ca_pem = BufReader::new(ca_pem);
    let ref ca = pemfile::certs(&mut ca_pem).unwrap()[0];  //FIXME there are a lot of certs in certs, so you're probs only getting one right now
    let anchors = vec![
        trust_anchor_util::cert_der_as_trust_anchor (   //resources/certs is a PEM
            Input::from(&ca)
        ).unwrap()
    ];
    match x509_ctx.current_cert() {
        Some(x509) => {//{webpki::EndEntityCert::from(untrusted::Input::from(x509));},//webpki verify everything--just do it serially first,
                        //step 1: create a webpki cert
                        debug!("fingerprint: {:?}",
                            x509.fingerprint(MessageDigest::sha256()));
                        let der = x509.to_der().unwrap();
                        let cert = EndEntityCert::from(Input::from(&der)).unwrap();

                        //step 2: verify is valid tls cert
                        //FIXME not sure what inter_vec shoule be here
                        let inter_vec = get_inter_vec(x509_ctx);   //should probably be x509.chain()
                        let chain: Vec<Input> = inter_vec.iter()
                            .map(|c| Input::from(&c.clone().to_der().unwrap()))
                            .collect();
                        match cert.verify_is_valid_tls_server_cert(ALL_SIGALGS, &anchors, &chain, time::get_time()) {
                            Ok(_) => {debug!("valid tls cert");},
                            Err(err) => {debug!("not valid tls cert: {:?}", err); return false;},
                        };

                        //step 3: verify is valid for dns name
                        match cert.verify_is_valid_for_dns_name(Input::from(domain.as_bytes())) {
                            Ok(_) => {debug!("valid for domain");},
                            Err(err) => {debug!("not valid for domain: {:?}", err); return false;},
                        };
                        //step 4: verify_signature TODO
                    },
        None => ( return false ),
    };

    true
}*/

fn get_inter_vec(x509_ctx: &X509StoreContextRef) -> Vec<&X509Ref> {
    let mut inter_vec = vec!();
    match x509_ctx.chain() {
        //Some(chain) => vec!(),//.extend(chain),
        Some(chain) => inter_vec.extend(chain),
        None => (),
    };
    inter_vec
}
