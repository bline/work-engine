use crate::codec::{JsString, JsValue, digest, exact_fields, field};
use crate::{EpisodeError, EpisodeResult, ErrorKind};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Revision(pub String);

impl Revision {
    pub fn parse(value: &JsValue, label: &str) -> EpisodeResult<Self> {
        let text = value.as_text()?.to_string_checked()?;
        if text.len() != 64
            || !text
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err(EpisodeError::new(format!(
                "{label} must be lowercase SHA-256"
            )));
        }
        Ok(Self(text))
    }

    pub fn value(&self) -> JsValue {
        JsValue::text(&self.0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Identity {
    pub run_id: JsString,
    pub slice_number: u64,
    pub attempt_id: JsString,
    pub plan_version: JsString,
    pub review_obligation_id: JsString,
    pub review_episode_id: JsString,
}

impl Identity {
    pub fn parse(value: &JsValue) -> EpisodeResult<Self> {
        exact_fields(
            value,
            &[
                "runId",
                "sliceNumber",
                "attemptId",
                "planVersion",
                "reviewObligationId",
                "reviewEpisodeId",
            ],
            "review episode identity",
        )?;
        Ok(Self {
            run_id: field(value, "runId")?
                .nonempty_text("identity.runId")?
                .clone(),
            slice_number: field(value, "sliceNumber")?
                .as_safe_positive_integer("identity.sliceNumber")?,
            attempt_id: field(value, "attemptId")?
                .nonempty_text("identity.attemptId")?
                .clone(),
            plan_version: field(value, "planVersion")?
                .nonempty_text("identity.planVersion")?
                .clone(),
            review_obligation_id: field(value, "reviewObligationId")?
                .nonempty_text("identity.reviewObligationId")?
                .clone(),
            review_episode_id: field(value, "reviewEpisodeId")?
                .nonempty_text("identity.reviewEpisodeId")?
                .clone(),
        })
    }

    pub fn value(&self) -> JsValue {
        JsValue::object([
            ("runId", JsValue::String(self.run_id.clone())),
            ("sliceNumber", JsValue::Number(self.slice_number as f64)),
            ("attemptId", JsValue::String(self.attempt_id.clone())),
            ("planVersion", JsValue::String(self.plan_version.clone())),
            (
                "reviewObligationId",
                JsValue::String(self.review_obligation_id.clone()),
            ),
            (
                "reviewEpisodeId",
                JsValue::String(self.review_episode_id.clone()),
            ),
        ])
    }

    pub fn key(&self) -> Revision {
        Revision(digest(&self.value()))
    }
}

pub fn validate_reference(value: &JsValue, label: &str) -> EpisodeResult<()> {
    exact_fields(
        value,
        &["owner", "reference", "revision", "sha256", "freshness"],
        label,
    )?;
    for key in ["owner", "reference", "revision", "freshness"] {
        field(value, key)?.nonempty_text(&format!("{label}.{key}"))?;
    }
    Revision::parse(field(value, "sha256")?, &format!("{label}.sha256"))?;
    Ok(())
}

#[derive(Clone, Debug, PartialEq)]
pub struct Writer {
    pub actor_id: JsString,
    pub provider: JsString,
    pub generation: u64,
    pub runtime_session: JsValue,
}

impl Writer {
    pub fn parse(value: &JsValue) -> EpisodeResult<Self> {
        exact_fields(
            value,
            &["actorId", "provider", "generation", "runtimeSession"],
            "review episode writer",
        )?;
        let session = field(value, "runtimeSession")?;
        validate_reference(session, "review episode writer.runtimeSession")?;
        Ok(Self {
            actor_id: field(value, "actorId")?
                .nonempty_text("writer.actorId")?
                .clone(),
            provider: field(value, "provider")?
                .nonempty_text("writer.provider")?
                .clone(),
            generation: field(value, "generation")?
                .as_safe_positive_integer("writer.generation")?,
            runtime_session: session.clone(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct Authority {
    pub(crate) value: JsValue,
    pub(crate) identity: Identity,
    pub(crate) writer: Writer,
    pub(crate) initial_subject: JsValue,
    pub(crate) predecessor_revision: Option<Revision>,
}

impl Authority {
    pub fn parse(value: JsValue) -> EpisodeResult<Self> {
        Self::parse_inner(value).map_err(|error| error.with_kind(ErrorKind::Authority))
    }

    fn parse_inner(value: JsValue) -> EpisodeResult<Self> {
        exact_fields(
            &value,
            &[
                "schemaVersion",
                "grantId",
                "identity",
                "source",
                "writer",
                "readers",
                "initialSubject",
                "predecessorRevision",
            ],
            "review episode authority",
        )?;
        if field(&value, "schemaVersion")? != &JsValue::Number(1.0) {
            return Err(EpisodeError::new(
                "review episode authority schema version is invalid",
            ));
        }
        field(&value, "grantId")?.nonempty_text("authority.grantId")?;
        validate_reference(field(&value, "source")?, "review episode authority.source")?;
        let identity = Identity::parse(field(&value, "identity")?)?;
        let writer = Writer::parse(field(&value, "writer")?)?;
        let readers = field(&value, "readers")?.as_array()?;
        if readers.is_empty()
            || readers
                .iter()
                .any(|reader| reader.nonempty_text("authority.reader").is_err())
        {
            return Err(EpisodeError::new(
                "review episode authority.readers are invalid",
            ));
        }
        let mut unique = std::collections::BTreeSet::new();
        if readers
            .iter()
            .any(|reader| !unique.insert(reader.as_text().expect("checked reader")))
        {
            return Err(EpisodeError::new(
                "review episode authority.readers are invalid",
            ));
        }
        let initial_subject = field(&value, "initialSubject")?.clone();
        validate_reference(&initial_subject, "review episode authority.initialSubject")?;
        let predecessor_revision = match field(&value, "predecessorRevision")? {
            JsValue::Null => None,
            other => Some(Revision::parse(
                other,
                "review episode authority.predecessorRevision",
            )?),
        };
        if (writer.generation == 1 && predecessor_revision.is_some())
            || (writer.generation > 1 && predecessor_revision.is_none())
        {
            return Err(EpisodeError::new(
                "review episode authority predecessor is invalid",
            ));
        }
        Ok(Self {
            value,
            identity,
            writer,
            initial_subject,
            predecessor_revision,
        })
    }

    pub fn binding(&self) -> JsValue {
        JsValue::object([
            (
                "grantId",
                self.value
                    .get("grantId")
                    .expect("validated authority")
                    .clone(),
            ),
            (
                "source",
                self.value
                    .get("source")
                    .expect("validated authority")
                    .clone(),
            ),
            ("manifestRevision", JsValue::text(&digest(&self.value))),
            (
                "readers",
                self.value
                    .get("readers")
                    .expect("validated authority")
                    .clone(),
            ),
        ])
    }

    pub fn writer_value(&self) -> JsValue {
        self.value
            .get("writer")
            .expect("validated authority")
            .clone()
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
    pub fn initial_subject(&self) -> &JsValue {
        &self.initial_subject
    }
    pub fn predecessor_revision(&self) -> Option<&Revision> {
        self.predecessor_revision.as_ref()
    }
}
