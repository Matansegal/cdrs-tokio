use crate::consistency::Consistency;
use crate::frame::*;
use crate::query::{Query, QueryParams, QueryValues};
use crate::types::*;
use std::io::Cursor;

/// Structure which represents body of Query request
#[derive(Debug)]
pub struct BodyReqQuery {
    /// Query string.
    pub query: CStringLong,
    /// Query parameters.
    pub query_params: QueryParams,
}

impl BodyReqQuery {
    #[allow(clippy::too_many_arguments)]
    fn new(
        query: String,
        consistency: Consistency,
        values: Option<QueryValues>,
        with_names: bool,
        page_size: Option<i32>,
        paging_state: Option<CBytes>,
        serial_consistency: Option<Consistency>,
        timestamp: Option<i64>,
        is_idempotent: bool,
    ) -> BodyReqQuery {
        BodyReqQuery {
            query: CStringLong::new(query),
            query_params: QueryParams {
                consistency,
                with_names,
                values,
                page_size,
                paging_state,
                serial_consistency,
                timestamp,
                is_idempotent,
                keyspace: None,
                token: None,
                routing_key: None,
            },
        }
    }
}

impl Serialize for BodyReqQuery {
    #[inline]
    fn serialize(&self, cursor: &mut Cursor<&mut Vec<u8>>) {
        self.query.serialize(cursor);
        self.query_params.serialize(cursor);
    }
}

impl Frame {
    #[allow(clippy::too_many_arguments)]
    pub fn new_req_query(
        query: String,
        consistency: Consistency,
        values: Option<QueryValues>,
        with_names: bool,
        page_size: Option<i32>,
        paging_state: Option<CBytes>,
        serial_consistency: Option<Consistency>,
        timestamp: Option<i64>,
        flags: Flags,
        is_idempotent: bool,
        version: Version,
    ) -> Frame {
        let direction = Direction::Request;
        let opcode = Opcode::Query;
        let body = BodyReqQuery::new(
            query,
            consistency,
            values,
            with_names,
            page_size,
            paging_state,
            serial_consistency,
            timestamp,
            is_idempotent,
        );

        Frame::new(
            version,
            direction,
            flags,
            opcode,
            body.serialize_to_vec(),
            None,
            vec![],
        )
    }

    #[inline]
    pub fn new_query(query: Query, flags: Flags, version: Version) -> Frame {
        Frame::new_req_query(
            query.query,
            query.params.consistency,
            query.params.values,
            query.params.with_names,
            query.params.page_size,
            query.params.paging_state,
            query.params.serial_consistency,
            query.params.timestamp,
            flags,
            query.params.is_idempotent,
            version,
        )
    }
}