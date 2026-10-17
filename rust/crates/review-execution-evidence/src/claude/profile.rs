pub use review_execution_custody::claude::profile::{
    PinnedProgram, Rp0Error, TrustedClaudeProfile,
};
pub(crate) use review_execution_custody::codec::sha256;

#[derive(Clone, Debug)]
pub struct TrustedRp0Profile {
    pub(crate) bwrap: PinnedProgram,
    pub(crate) cbm: PinnedProgram,
    pub(crate) cbm_launcher: PinnedProgram,
    pub(crate) cbm_client: PinnedProgram,
}

impl TrustedRp0Profile {
    pub fn load(
        bwrap: PinnedProgram,
        cbm: PinnedProgram,
        cbm_launcher: PinnedProgram,
        cbm_client: PinnedProgram,
    ) -> Result<Self, Rp0Error> {
        let profile = Self {
            bwrap,
            cbm,
            cbm_launcher,
            cbm_client,
        };
        profile.verify()?;
        Ok(profile)
    }

    pub(crate) fn verify(&self) -> Result<(), Rp0Error> {
        self.bwrap.verify()?;
        self.cbm.verify()?;
        self.cbm_launcher.verify()?;
        self.cbm_client.verify()
    }
}
