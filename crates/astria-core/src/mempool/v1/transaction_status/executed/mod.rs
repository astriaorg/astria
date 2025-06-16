use base64::{
    engine::general_purpose::STANDARD,
    Engine,
};
use tendermint::abci::{
    types::ExecTxResult,
    v0_34,
    v0_37,
    Event,
    EventAttribute,
};

use crate::{
    generated::tendermint::abci as Raw,
    Protobuf,
};

pub enum ExecTxResultError {}

impl Protobuf for ExecTxResult {
    type Error = ExecTxResultError;
    type Raw = Raw::ExecTxResult;

    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        let Self::Raw {
            code,
            data,
            log,
            info,
            gas_wanted,
            gas_used,
            events,
            codespace,
        } = raw;

        let events = events
            .iter()
            .map(Event::try_from_raw_ref)
            .collect::<Result<Vec<_>, _>>()
            .expect("Failed to convert raw event(an infallible process)");
        let result = ExecTxResult {
            code: (*code).into(),
            data: data.clone(),
            log: log.clone(),
            info: info.clone(),
            gas_wanted: *gas_wanted,
            gas_used: *gas_used,
            events,
            codespace: codespace.clone(),
        };
        Ok(result)
    }

    fn try_from_raw(raw: Self::Raw) -> Result<Self, Self::Error> {
        let Self::Raw {
            code,
            data,
            log,
            info,
            gas_wanted,
            gas_used,
            events,
            codespace,
        } = raw;

        let events = events
            .into_iter()
            .map(Event::try_from_raw)
            .collect::<Result<Vec<_>, _>>()
            .expect("Failed to convert raw event(an infallible process)");
        let result = ExecTxResult {
            code: code.into(),
            data,
            log,
            info,
            gas_wanted,
            gas_used,
            events,
            codespace,
        };
        Ok(result)
    }

    fn to_raw(&self) -> Self::Raw {
        let Self {
            code,
            data,
            log,
            info,
            gas_wanted,
            gas_used,
            events,
            codespace,
        } = self;
        let events = events.iter().map(Event::to_raw).collect::<Vec<_>>();
        Raw::ExecTxResult {
            code: (*code).into(),
            data: data.clone(),
            log: log.clone(),
            info: info.clone(),
            gas_wanted: *gas_wanted,
            gas_used: *gas_used,
            events,
            codespace: codespace.clone(),
        }
    }

    fn into_raw(self) -> Self::Raw {
        let Self {
            code,
            data,
            log,
            info,
            gas_wanted,
            gas_used,
            events,
            codespace,
        } = self;
        let events = events.into_iter().map(Event::into_raw).collect::<Vec<_>>();
        Raw::ExecTxResult {
            code: code.into(),
            data,
            log,
            info,
            gas_wanted,
            gas_used,
            events,
            codespace,
        }
    }
}

#[derive(Debug)]
pub struct EventError {}

impl Protobuf for Event {
    type Error = EventError;
    type Raw = Raw::Event;

    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        let Self::Raw {
            kind,
            attributes,
        } = raw;
        let attributes = attributes
            .iter()
            .map(EventAttribute::try_from_raw_ref)
            .collect::<Result<Vec<_>, _>>()
            .expect("Failed to convert raw attribute(an infallible process)");
        Ok(Event {
            kind: kind.clone(),
            attributes,
        })
    }

    fn try_from_raw(raw: Self::Raw) -> Result<Self, Self::Error> {
        let Self::Raw {
            kind,
            attributes,
        } = raw;
        let attributes = attributes
            .into_iter()
            .map(EventAttribute::try_from_raw)
            .collect::<Result<Vec<_>, _>>()
            .expect("Failed to convert raw attribute (an infallible process)");
        Ok(Event {
            kind,
            attributes,
        })
    }

    fn to_raw(&self) -> Self::Raw {
        let Self {
            kind,
            attributes,
        } = self;
        let attributes = attributes
            .iter()
            .map(EventAttribute::to_raw)
            .collect::<Vec<_>>();
        Raw::Event {
            kind: kind.clone(),
            attributes,
        }
    }

    fn into_raw(self) -> Self::Raw {
        let Self {
            kind,
            attributes,
        } = self;
        let attributes = attributes
            .into_iter()
            .map(EventAttribute::into_raw)
            .collect::<Vec<_>>();
        Raw::Event {
            kind,
            attributes,
        }
    }
}

#[derive(Debug)]
pub struct EventAttributeError {}

impl Protobuf for EventAttribute {
    type Error = EventAttributeError;
    type Raw = Raw::EventAttribute;

    fn try_from_raw_ref(raw: &Self::Raw) -> Result<Self, Self::Error> {
        let Self::Raw {
            key,
            value,
            index,
        } = raw;

        Ok(EventAttribute::V037(v0_37::EventAttribute {
            key: key.clone(),
            value: value.clone(),
            index: *index,
        }))
    }

    fn try_from_raw(raw: Self::Raw) -> Result<Self, Self::Error> {
        let Self::Raw {
            key,
            value,
            index,
        } = raw;

        Ok(EventAttribute::V037(v0_37::EventAttribute {
            key,
            value,
            index,
        }))
    }

    fn to_raw(&self) -> Self::Raw {
        match self {
            EventAttribute::V037(v0_37::EventAttribute {
                key,
                value,
                index,
            }) => Raw::EventAttribute {
                key: key.clone(),
                value: value.clone(),
                index: *index,
            },
            EventAttribute::V034(v0_34::EventAttribute {
                key,
                value,
                index,
            }) => Raw::EventAttribute {
                key: STANDARD.encode(key),
                value: STANDARD.encode(value),
                index: *index,
            },
        }
    }

    fn into_raw(self) -> Self::Raw {
        match self {
            EventAttribute::V037(v0_37::EventAttribute {
                key,
                value,
                index,
            }) => Raw::EventAttribute {
                key,
                value,
                index,
            },
            EventAttribute::V034(v0_34::EventAttribute {
                key,
                value,
                index,
            }) => Raw::EventAttribute {
                key: STANDARD.encode(key),
                value: STANDARD.encode(value),
                index,
            },
        }
    }
}
