use std::time::Duration;

use bytes::Bytes;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CreateOfferError {
    #[error("Requwest error {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("Missing location header")]
    MissingLocation,
    #[error("Invalid localtion value")]
    InvalidLocation,
    #[error("Invalid status code")]
    InvalidStatus,
    #[error("Invalid body")]
    InvalidBody,
}

pub struct CreatedOffer {
    pub endpoint: String,
    pub sdp: Bytes,
}

pub async fn create_offer(gateway: &str, token: &str) -> Result<CreatedOffer, CreateOfferError> {
    let res = reqwest::ClientBuilder::new()
        .timeout(Duration::from_secs(3))
        .build()
        .expect("Should create client")
        .post(&format!("{gateway}/rtpengine/offer"))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;

    if res.status().as_u16() == 201 {
        let endpoint = res
            .headers()
            .get("Location")
            .ok_or(CreateOfferError::MissingLocation)?;
        Ok(CreatedOffer {
            endpoint: endpoint
                .to_str()
                .map_err(|_e| CreateOfferError::InvalidLocation)?
                .to_string(),
            sdp: res.bytes().await?,
        })
    } else {
        Err(CreateOfferError::InvalidStatus)
    }
}
