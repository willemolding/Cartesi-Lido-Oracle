//! Request a cartesi tower service must handle
use crate::messages::{AdvanceStateMetadata, RollupRequest};

#[derive(Debug)]
pub enum Request {
    AdvanceState {
        metadata: AdvanceStateMetadata,
        payload: Vec<u8>,
    },
    InspectState {
        payload: Vec<u8>,
    },
}

impl TryFrom<RollupRequest> for Request {
    type Error = hex::FromHexError;

    fn try_from(request: RollupRequest) -> Result<Self, Self::Error> {
        match request {
            RollupRequest::AdvanceState { data } => Ok(Request::AdvanceState {
                metadata: data.metadata,
                payload: hex::decode(data.payload.trim_start_matches("0x"))?,
            }),
            RollupRequest::InspectState { data } => Ok(Request::InspectState {
                payload: hex::decode(data.payload.trim_start_matches("0x"))?,
            }),
        }
    }
}

impl TryFrom<crate::inputs_query::InputsQueryInputsEdgesNode> for Request {
    type Error = hex::FromHexError;

    fn try_from(
        value: crate::inputs_query::InputsQueryInputsEdgesNode,
    ) -> Result<Self, Self::Error> {
        Ok(Self::AdvanceState {
            metadata: AdvanceStateMetadata {
                msg_sender: ethereum_types::Address::from_slice(&hex::decode(
                    value.msg_sender.trim_start_matches("0x"),
                )?),
                prev_randao: String::new(), // TODO: not sure what to do here..
                input_index: value.index.try_into().unwrap(),
                block_number: 0,
                block_timestamp: 0,
                chain_id: 0,
                app_contract: String::new(),
            },
            payload: hex::decode(value.payload.trim_start_matches("0x"))?,
        })
    }
}
