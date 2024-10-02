use ezk_sip_core::{Endpoint, IncomingRequest, Layer, LayerKey, MayTake};
use ezk_sip_types::{header::typed::Contact, Method};
use ezk_sip_ua::{dialog::DialogLayer, invite::InviteLayer};

use crate::sip::IncallValidator;

/// Custom layer which we use to accept incoming invites
pub struct InviteAcceptLayer<V> {
    contact: Contact,
    dialog_layer: LayerKey<DialogLayer>,
    invite_layer: LayerKey<InviteLayer>,
    validator: V,
}

impl<V: IncallValidator> InviteAcceptLayer<V> {
    pub fn new(contact: Contact, dialog_layer: LayerKey<DialogLayer>, invite_layer: LayerKey<InviteLayer>, validator: V) -> Self {
        Self {
            contact,
            dialog_layer,
            invite_layer,
            validator,
        }
    }
}

#[async_trait::async_trait]
impl<V: IncallValidator> Layer for InviteAcceptLayer<V> {
    fn name(&self) -> &'static str {
        "invite-accept-layer"
    }

    async fn receive(&self, endpoint: &Endpoint, request: MayTake<'_, IncomingRequest>) {
        let invite = if request.line.method == Method::INVITE {
            request.take()
        } else {
            return;
        };

        todo!()
    }
}
