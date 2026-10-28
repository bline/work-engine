//! Bounded Responses request, observation, credential state, and distinct transports.
//!
//! The local transport remains loopback-only. The public host has fixed HTTPS
//! authority and requires explicit operator grants before one-shot inference.
//! This crate does not assign lifecycle identity or semantic adequacy.

mod auth;
mod direct;
mod journal;
mod local_http;
mod public_host;
mod public_https;
mod request;
mod stream;

#[cfg(feature = "auth-test-faults")]
pub use auth::StoreFault;
pub use auth::{
    AuthError, AuthStart, AuthStore, BearerAccess, CallbackExchange, Issuer, Readiness,
    RefreshFailure, RefreshGrant, RegistrationRef, RevocationGrant, RevocationOutcome,
    SavedRegistration, TokenResponse,
};
pub use direct::{
    DirectAdapter, DirectEndpoint, DirectExchange, DirectProfile, LoopbackEndpoint,
    PublicHttpsEndpoint,
};
pub use journal::{
    AttemptJournal, AttemptRecord, AttemptState, ContentTypeCapture, ContentTypeMeta,
    HttpErrorShape, HttpEvidence, JournalError,
};
pub use local_http::{
    DiscoveredModel, LocalEndpoint, LocalHttpTransport, LocalTransportError,
    RefreshExchangeEvidence, RefreshExchangeOutcome, SELECTED_REASONING_PROFILE_REVISION,
};
pub use public_host::{
    ActionGrant, DiagnosticAdmission, DiagnosticGrant, DiagnosticProjection, DirectAdmissionStatus,
    DirectSendGrant, HostError, ModelObservation, PublicHost, PublicHostEndpoint, SmokeGrant,
    SmokeProjection, direct_identity_key,
};
pub(crate) use public_https::PublicHttps;
pub use public_https::PublicHttpsError;
pub use request::{AssembledRequest, Bounds, RequestError, ToolFreeProfile, assemble_request};
pub use stream::{
    ObservedTerminal, StreamError, StreamObservation, StreamObserver, TokenCounts,
    UsageObservation, UsageSemantics,
};
