use crate::{CampaignError, Result};
use std::os::unix::net::UnixStream;

pub(crate) fn authenticated_peer(stream: &UnixStream, installed_uid: u32) -> Result<u32> {
    let cred = rustix::net::sockopt::socket_peercred(stream)
        .map_err(|e| CampaignError::Root(format!("socket peer credentials: {e}")))?;
    let observed = cred.uid.as_raw();
    if observed != installed_uid {
        return Err(CampaignError::Conflict(
            "operator peer UID differs from installed authority".into(),
        ));
    }
    Ok(observed)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn connected_peer_uid_is_used_instead_of_a_request_actor() {
        let (a, _b) = UnixStream::pair().unwrap();
        let uid = rustix::process::getuid().as_raw();
        assert_eq!(authenticated_peer(&a, uid).unwrap(), uid);
        assert!(
            authenticated_peer(&a, uid.wrapping_add(1))
                .unwrap_err()
                .to_string()
                .contains("operator peer UID differs from installed authority")
        );
    }
}
