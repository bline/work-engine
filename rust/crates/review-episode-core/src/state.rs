use crate::codec::{
    JsString, JsValue, canonical_json, digest, exact_fields, field, parse_json, string_is,
};
use crate::identity::{Identity, Revision, Writer, validate_reference};
use crate::implementation_review_v1::ReviewResult;
use crate::{EpisodeError, EpisodeResult, ErrorKind};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    Active,
    Uncertain,
    Retired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    InitialReview,
    Remediation,
    ReEvaluation,
    Reported,
    EvidenceUnestablished,
}

#[derive(Clone, Debug)]
pub struct ReviewEpisodeState {
    pub(crate) value: JsValue,
    pub(crate) identity: Identity,
    pub(crate) writer: Writer,
    pub(crate) status: Status,
    pub(crate) phase: Phase,
    pub(crate) revision: Revision,
}

impl ReviewEpisodeState {
    /// Construct a new state from semantic fields. The digest excludes `revision`.
    pub fn publish(semantic: JsValue) -> EpisodeResult<Self> {
        Self::publish_inner(semantic).map_err(|error| error.with_kind(ErrorKind::StateIntegrity))
    }

    fn publish_inner(semantic: JsValue) -> EpisodeResult<Self> {
        if semantic.get("revision").is_some() {
            return Err(EpisodeError::new("semantic state must exclude revision"));
        }
        let (identity, writer, status, phase) = validate_semantic(&semantic)?;
        let revision = Revision(digest(&semantic));
        let mut map = semantic.as_object()?.clone();
        map.insert(JsString::new("revision"), revision.value());
        Ok(Self {
            value: JsValue::Object(map),
            identity,
            writer,
            status,
            phase,
            revision,
        })
    }

    /// Read historical stored JSON without upgrading its schema or rewriting bytes.
    pub fn from_stored_json(source: &str) -> EpisodeResult<Self> {
        Self::from_stored_json_inner(source)
            .map_err(|error| error.with_kind(ErrorKind::StateIntegrity))
    }

    fn from_stored_json_inner(source: &str) -> EpisodeResult<Self> {
        let value = parse_json(source)?;
        if canonical_json(&value) != source {
            return Err(EpisodeError::new(
                "stored review episode JSON is not canonical",
            ));
        }
        let actual_revision =
            Revision::parse(field(&value, "revision")?, "stored review episode revision")?;
        let mut semantic = value.as_object()?.clone();
        semantic.remove(&JsString::new("revision"));
        let semantic = JsValue::Object(semantic);
        let (identity, writer, status, phase) = validate_semantic(&semantic)?;
        if digest(&semantic) != actual_revision.0 {
            return Err(EpisodeError::new(
                "stored review episode revision failed integrity check",
            ));
        }
        Ok(Self {
            value,
            identity,
            writer,
            status,
            phase,
            revision: actual_revision,
        })
    }

    pub fn semantic(&self) -> JsValue {
        let mut map = self.value.as_object().expect("validated state").clone();
        map.remove(&JsString::new("revision"));
        JsValue::Object(map)
    }

    pub fn value(&self) -> &JsValue {
        &self.value
    }
    pub fn identity(&self) -> &Identity {
        &self.identity
    }
    pub fn writer(&self) -> &Writer {
        &self.writer
    }
    pub fn status(&self) -> Status {
        self.status
    }
    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn revision(&self) -> &Revision {
        &self.revision
    }

    pub fn get(&self, key: &str) -> &JsValue {
        self.value.get(key).expect("validated state field")
    }

    pub fn schema_version(&self) -> u8 {
        if self.get("schemaVersion") == &JsValue::Number(2.0) {
            2
        } else {
            1
        }
    }

    pub fn current_result(&self) -> EpisodeResult<Option<ReviewResult>> {
        match self.get("currentResult") {
            JsValue::Null => Ok(None),
            value => Ok(Some(ReviewResult::parse(value.clone())?)),
        }
    }

    pub fn handled(&self, id: &JsString) -> Option<&JsValue> {
        self.get("handledTransitions")
            .as_object()
            .expect("validated transitions")
            .get(id)
    }
}

fn validate_semantic(value: &JsValue) -> EpisodeResult<(Identity, Writer, Status, Phase)> {
    let version = match field(value, "schemaVersion")? {
        JsValue::Number(number) if *number == 1.0 => 1,
        JsValue::Number(number) if *number == 2.0 => 2,
        _ => {
            return Err(EpisodeError::new(
                "review episode state schema version is invalid",
            ));
        }
    };
    let mut fields = vec![
        "schemaVersion",
        "identity",
        "authority",
        "writer",
        "status",
        "phase",
        "subject",
        "currentResult",
        "unresolvedQuestions",
        "pendingAction",
        "handledTransitions",
        "continuity",
        "uncertainty",
        "retirement",
    ];
    if version == 2 {
        fields.push("evidenceAdmissions");
    }
    exact_fields(value, &fields, "review episode state")?;
    let identity = Identity::parse(field(value, "identity")?)?;
    let authority = field(value, "authority")?;
    exact_fields(
        authority,
        &["grantId", "source", "manifestRevision", "readers"],
        "review episode authority binding",
    )?;
    field(authority, "grantId")?.nonempty_text("authority binding.grantId")?;
    validate_reference(
        field(authority, "source")?,
        "review episode authority binding.source",
    )?;
    Revision::parse(
        field(authority, "manifestRevision")?,
        "authority binding.manifestRevision",
    )?;
    validate_text_array(field(authority, "readers")?, "authority binding.readers")?;
    let writer = Writer::parse(field(value, "writer")?)?;
    let status = match field(value, "status")? {
        item if string_is(item, "active") => Status::Active,
        item if string_is(item, "uncertain") => Status::Uncertain,
        item if string_is(item, "retired") => Status::Retired,
        _ => return Err(EpisodeError::new("review episode status is invalid")),
    };
    let phase = match field(value, "phase")? {
        item if string_is(item, "initial_review") => Phase::InitialReview,
        item if string_is(item, "remediation") => Phase::Remediation,
        item if string_is(item, "re_evaluation") => Phase::ReEvaluation,
        item if string_is(item, "reported") => Phase::Reported,
        item if string_is(item, "evidence_unestablished") => Phase::EvidenceUnestablished,
        _ => return Err(EpisodeError::new("review episode phase is invalid")),
    };
    validate_reference(field(value, "subject")?, "review episode subject")?;
    if field(value, "currentResult")? != &JsValue::Null {
        ReviewResult::parse(field(value, "currentResult")?.clone())?;
    }
    if version == 2 {
        for admission in field(value, "evidenceAdmissions")?.as_array()? {
            validate_admission(admission)?;
        }
    }
    validate_text_array(
        field(value, "unresolvedQuestions")?,
        "review episode unresolvedQuestions",
    )?;
    field(value, "pendingAction")?.nonempty_text("review episode pendingAction")?;
    for (id, revision) in field(value, "handledTransitions")?.as_object()? {
        if !id.is_nonempty_text() {
            return Err(EpisodeError::new("review episode transition id is invalid"));
        }
        Revision::parse(revision, "review episode transition digest")?;
    }
    let continuity = field(value, "continuity")?;
    if ![
        "fresh_initial",
        "same_session",
        "reconstructed_continuation",
    ]
    .iter()
    .any(|item| string_is(continuity, item))
    {
        return Err(EpisodeError::new("review episode continuity is invalid"));
    }
    let uncertainty = field(value, "uncertainty")?;
    if status == Status::Uncertain {
        exact_fields(
            uncertainty,
            &["reason", "reconciliationAction"],
            "review episode uncertainty",
        )?;
        for key in ["reason", "reconciliationAction"] {
            field(uncertainty, key)?.nonempty_text(&format!("uncertainty.{key}"))?;
        }
    } else if uncertainty != &JsValue::Null {
        return Err(EpisodeError::new(
            "only uncertain review episode state may contain uncertainty",
        ));
    }
    let retirement = field(value, "retirement")?;
    if status == Status::Retired {
        exact_fields(
            retirement,
            &["outcome", "reason", "protectedReferences"],
            "review episode retirement",
        )?;
        for key in ["outcome", "reason"] {
            field(retirement, key)?.nonempty_text(&format!("retirement.{key}"))?;
        }
        let refs = field(retirement, "protectedReferences")?.as_array()?;
        if refs.is_empty() {
            return Err(EpisodeError::new(
                "review episode retirement requires protected references",
            ));
        }
        for item in refs {
            validate_reference(item, "review episode retirement.protectedReferences")?;
        }
    } else if retirement != &JsValue::Null {
        return Err(EpisodeError::new(
            "only retired review episode state may contain retirement",
        ));
    }
    Ok((identity, writer, status, phase))
}

pub fn validate_text_array(value: &JsValue, label: &str) -> EpisodeResult<()> {
    for item in value.as_array()? {
        item.nonempty_text(label)?;
    }
    Ok(())
}

pub fn validate_admission(value: &JsValue) -> EpisodeResult<()> {
    exact_fields(
        value,
        &[
            "claimRevisionRef",
            "establishmentRef",
            "observationRef",
            "consumptionRef",
            "status",
            "boundary",
            "consumer",
        ],
        "review episode evidence admission",
    )?;
    for key in ["claimRevisionRef", "establishmentRef", "consumptionRef"] {
        validate_reference(field(value, key)?, &format!("review episode {key}"))?;
    }
    if field(value, "observationRef")? != &JsValue::Null {
        validate_reference(
            field(value, "observationRef")?,
            "review episode observationRef",
        )?;
    }
    if !["established", "false", "unestablished"]
        .iter()
        .any(|item| string_is(field(value, "status").expect("field validated"), item))
    {
        return Err(EpisodeError::new(
            "review episode evidence status is invalid",
        ));
    }
    field(value, "boundary")?.nonempty_text("review episode evidence boundary")?;
    field(value, "consumer")?.nonempty_text("review episode evidence consumer")?;
    Ok(())
}
