use std::{io, net::SocketAddr};

use ezk_sip_core::{transport::udp::Udp, Endpoint, LayerKey};
use ezk_sip_types::{
    header::typed::Contact,
    uri::{sip::SipUri, NameAddr},
};
use ezk_sip_ua::{dialog::DialogLayer, invite::InviteLayer};
use incoming::InviteAcceptLayer;
use thiserror::Error;

use crate::{
    address_book::AddressBookStorage,
    protocol::{SipAuth, StreamingInfo},
};

mod incoming;
mod outgoing;

pub use outgoing::{SipOutgoingCall, SipOutgoingCallError, SipOutgoingCallOut};

use super::MediaApi;

#[derive(Debug, Error)]
pub enum SipServerError {
    #[error("Unknown error")]
    Unknown,
}

pub struct SipServer {
    endpoint: Endpoint,
    contact: Contact,
    dialog_layer: LayerKey<DialogLayer>,
    invite_layer: LayerKey<InviteLayer>,
}

impl SipServer {
    pub async fn new(addr: SocketAddr, address_book: AddressBookStorage) -> io::Result<Self> {
        let mut builder = Endpoint::builder();

        let dialog_layer = builder.add_layer(DialogLayer::default());
        let invite_layer = builder.add_layer(InviteLayer::default());

        let contact: SipUri = format!("sip:atm0s@{}", addr).parse().expect("Should parse");
        let contact = Contact::new(NameAddr::uri(contact));
        builder.add_layer(InviteAcceptLayer::new(contact.clone(), dialog_layer, invite_layer, address_book));

        Udp::spawn(&mut builder, addr).await?;

        // Build endpoint to start the SIP Stack
        let endpoint = builder.build();

        Ok(Self {
            endpoint,
            contact,
            dialog_layer,
            invite_layer,
        })
    }

    pub fn make_call(&self, media_api: MediaApi, from: &str, to: &str, auth: Option<SipAuth>, stream: StreamingInfo) -> Result<SipOutgoingCall, SipOutgoingCallError> {
        SipOutgoingCall::new(media_api, self.endpoint.clone(), self.dialog_layer, self.invite_layer, from, to, self.contact.clone(), auth, stream)
    }
}
