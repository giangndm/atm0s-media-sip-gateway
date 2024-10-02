use std::time::Duration;

use bytes::Bytes;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RtpEngineError {
    #[error("Requwest error {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("Missing location header")]
    MissingLocation,
    #[error("Invalid localtion value")]
    InvalidLocation,
    #[error("Invalid status code ({0})")]
    InvalidStatus(u16),
    #[error("Invalid body")]
    InvalidBody,
}

pub struct RtpEngineOffer {
    gateway: String,
    token: String,
    offer: Option<(String, Bytes)>,
}

impl RtpEngineOffer {
    pub fn new(gateway: &str, token: &str) -> Self {
        Self {
            gateway: gateway.to_string(),
            token: token.to_string(),
            offer: None,
        }
    }

    pub fn sdp(&self) -> Option<Bytes> {
        self.offer.as_ref().map(|(_, sdp)| sdp.clone())
    }

    pub async fn create_offer(&mut self) -> Result<Bytes, RtpEngineError> {
        assert!(self.offer.is_none(), "should not call create_offer twice");
        log::info!("[RtpEngineOffer] creating offer");
        let res = reqwest::ClientBuilder::new()
            .timeout(Duration::from_secs(3))
            .build()
            .expect("Should create client")
            .post(&format!("{}/rtpengine/offer", self.gateway))
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await?;

        let status = res.status().as_u16();

        if status == 201 {
            let endpoint = res.headers().get("Location").ok_or(RtpEngineError::MissingLocation)?;
            let location = endpoint.to_str().map_err(|_e| RtpEngineError::InvalidLocation)?.to_string();
            let sdp = res.bytes().await?;
            log::info!("[RtpEngineOffer] created offer {location}");
            self.offer = Some((location, sdp.clone()));
            Ok(sdp)
        } else {
            log::error!("[RtpEngineOffer] create offer error {status}");
            Err(RtpEngineError::InvalidStatus(status))
        }
    }

    pub async fn set_answer(&mut self, sdp: Bytes) -> Result<(), RtpEngineError> {
        let (location, _) = self.offer.as_ref().expect("should call after create_offer success");
        let url = format!("{}{}", self.gateway, location);
        log::info!("[RtpEngineOffer] sending answer {url}");

        let res = reqwest::ClientBuilder::new()
            .timeout(Duration::from_secs(3))
            .build()
            .expect("Should create client")
            .patch(&url)
            .header("Content-Type", "application/sdp")
            .body(sdp)
            .send()
            .await?;

        let status = res.status().as_u16();
        if status == 200 {
            log::info!("[RtpEngineOffer] sent answer {url}");
            Ok(())
        } else {
            log::error!("[RtpEngineOffer] send answer error {url} {status}");
            Err(RtpEngineError::InvalidStatus(status))
        }
    }
}

impl Drop for RtpEngineOffer {
    fn drop(&mut self) {
        if let Some((location, _)) = self.offer.take() {
            let url = format!("{}{}", self.gateway, location);
            tokio::spawn(async move {
                log::info!("[RtpEngineOffer] destroying {url}");
                let res = reqwest::ClientBuilder::new()
                    .timeout(Duration::from_secs(3))
                    .build()
                    .expect("Should create client")
                    .delete(&url)
                    .send()
                    .await?;

                let status = res.status().as_u16();
                if status == 200 {
                    log::info!("[RtpEngineOffer] destroyed {url}");
                    Ok(())
                } else {
                    log::error!("[RtpEngineOffer] destroy error {url} {status}");
                    Err(RtpEngineError::InvalidStatus(status))
                }
            });
        }
    }
}
